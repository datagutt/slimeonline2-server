//! Mail system handlers - mailbox, send mail, check receiver

use std::sync::Arc;
use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, warn, info};

use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageWriter, MessageType};
use crate::Server;
use crate::db;
use crate::constants::{MAX_MAIL_BODY, MAX_POINTS};

/// Handle MSG_MAILBOX (47)
/// Client requests mailbox contents (paginated)
/// 
/// Client sends:
/// - case (u8): 1 = get mailbox page, 2 = claim mail attachments
/// 
/// For case 1 (get mailbox):
/// - page (u8): page number (0-indexed)
/// 
/// For case 2 (claim attachments):
/// - mail_id (u16): mail to claim from
pub async fn handle_mailbox(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    
    let case = reader.read_u8()?;
    
    let character_id = session.read().await.character_id;
    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };
    
    match case {
        1 => {
            // Get mailbox page
            let page = reader.read_u8()? as i64;
            get_mailbox_page(server, char_id, page).await
        }
        2 => {
            // Claim mail attachments (item/points)
            let mail_id = reader.read_u16()? as i64;
            claim_mail_attachments(server, char_id, mail_id, session).await
        }
        _ => {
            debug!("Unknown mailbox case: {}", case);
            Ok(vec![])
        }
    }
}

/// Get a page of mails from the mailbox
async fn get_mailbox_page(
    server: &Arc<Server>,
    character_id: i64,
    page: i64,
) -> Result<Vec<Vec<u8>>> {
    // Get total mail count for pagination
    let total_count = db::get_mail_count(&server.db, character_id).await.unwrap_or(0);
    let total_pages = ((total_count + 4) / 5) as u8; // Ceiling division, 5 per page
    
    // Get mails for this page
    let mails = db::get_mailbox(&server.db, character_id, page).await.unwrap_or_default();
    
    // Build response
    // Format: case (1), page, total_pages, mail_count, then for each mail:
    //   mail_id (u16), sender_name (string), message (string), item_id (u16), points (u32), is_read (u8)
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::Mailbox.id())
        .write_u8(1) // case: mailbox response
        .write_u8(page as u8)
        .write_u8(total_pages)
        .write_u8(mails.len() as u8);
    
    for mail in &mails {
        writer.write_u16(mail.id as u16)
            .write_string(&mail.sender_name)
            .write_string(&mail.message)
            .write_u16(mail.item_id as u16)
            .write_u32(mail.points as u32)
            .write_u8(mail.is_read as u8);
    }
    
    debug!("Sending mailbox page {} with {} mails (total {} pages)", page, mails.len(), total_pages);
    
    Ok(vec![writer.into_bytes()])
}

/// Claim attachments (item/points) from a mail
async fn claim_mail_attachments(
    server: &Arc<Server>,
    character_id: i64,
    mail_id: i64,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    // Get the mail
    let mail = match db::get_mail(&server.db, mail_id, character_id).await {
        Ok(Some(m)) => m,
        Ok(None) => {
            debug!("Mail {} not found for character {}", mail_id, character_id);
            return Ok(vec![build_claim_failed_response()]);
        }
        Err(e) => {
            warn!("Failed to get mail: {}", e);
            return Ok(vec![build_claim_failed_response()]);
        }
    };
    
    // Check if there's anything to claim
    if mail.item_id == 0 && mail.points == 0 {
        debug!("Mail {} has nothing to claim", mail_id);
        return Ok(vec![build_claim_failed_response()]);
    }
    
    let mut responses = Vec::new();
    
    // Handle item attachment
    if mail.item_id > 0 {
        // Find empty item slot
        if let Ok(Some(inventory)) = db::get_inventory(&server.db, character_id).await {
            let items = inventory.items();
            let empty_slot = items.iter().position(|&id| id == 0);
            
            if let Some(slot_idx) = empty_slot {
                let slot = (slot_idx + 1) as u8;
                // Add item to inventory
                if let Err(e) = db::update_item_slot(&server.db, character_id, slot, mail.item_id as i16).await {
                    warn!("Failed to add item from mail: {}", e);
                    return Ok(vec![build_claim_failed_response()]);
                }
                
                // Send MSG_GET_ITEM to notify client
                let mut item_writer = MessageWriter::new();
                item_writer.write_u16(MessageType::GetItem.id())
                    .write_u8(slot)
                    .write_u16(mail.item_id as u16);
                responses.push(item_writer.into_bytes());
                
                debug!("Claimed item {} from mail {} to slot {}", mail.item_id, mail_id, slot);
            } else {
                warn!("No empty slot for item from mail {}", mail_id);
                // Continue - we might still claim points
            }
        }
    }
    
    // Handle points attachment
    if mail.points > 0 {
        let current_points = session.read().await.points;
        let new_points = (current_points as i64 + mail.points).min(MAX_POINTS as i64) as u32;
        
        // Update points in database
        if let Err(e) = db::update_points(&server.db, character_id, new_points as i64).await {
            warn!("Failed to add points from mail: {}", e);
            return Ok(vec![build_claim_failed_response()]);
        }
        
        // Update session
        {
            let mut session_guard = session.write().await;
            session_guard.points = new_points;
        }
        
        // Send points update to client (using MSG_POINT)
        let mut points_writer = MessageWriter::new();
        points_writer.write_u16(MessageType::Point.id())
            .write_u32(new_points);
        responses.push(points_writer.into_bytes());
        
        debug!("Claimed {} points from mail {}", mail.points, mail_id);
    }
    
    // Clear the attachments from the mail (or delete it entirely)
    if let Err(e) = db::clear_mail_attachments(&server.db, mail_id, character_id).await {
        warn!("Failed to clear mail attachments: {}", e);
    }
    
    // Mark mail as read
    let _ = db::mark_mail_read(&server.db, mail_id, character_id).await;
    
    // Send claim success response
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::Mailbox.id())
        .write_u8(2) // case: claim success
        .write_u16(mail_id as u16);
    responses.push(writer.into_bytes());
    
    Ok(responses)
}

/// Build claim failed response
fn build_claim_failed_response() -> Vec<u8> {
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::Mailbox.id())
        .write_u8(3); // case: claim failed
    writer.into_bytes()
}

/// Handle MSG_MAIL_SEND (78)
/// Send a mail to another player
/// 
/// Client sends:
/// - receiver_name (string)
/// - message (string)
/// - item_slot (u8): 0 = no item, 1-9 = item slot
/// - points (u32): points to attach
pub async fn handle_mail_send(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    
    let receiver_name = reader.read_string()?;
    let message = reader.read_string()?;
    let item_slot = reader.read_u8()?;
    let points = reader.read_u32()?;
    
    let (character_id, username) = {
        let session_guard = session.read().await;
        (session_guard.character_id, session_guard.username.clone())
    };
    
    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };
    
    let sender_name = username.unwrap_or_else(|| "Unknown".to_string());
    
    // Validate message length
    if message.len() > MAX_MAIL_BODY {
        return Ok(vec![build_mail_send_response(false)]);
    }
    
    // Find receiver
    let receiver = match db::find_character_by_username(&server.db, &receiver_name).await {
        Ok(Some(char)) => char,
        Ok(None) => {
            debug!("Mail send failed: receiver '{}' not found", receiver_name);
            return Ok(vec![build_mail_send_response(false)]);
        }
        Err(e) => {
            warn!("Mail send failed: database error: {}", e);
            return Ok(vec![build_mail_send_response(false)]);
        }
    };
    
    // Don't allow sending to self
    if receiver.id == char_id {
        warn!("Mail send failed: tried to send to self");
        return Ok(vec![build_mail_send_response(false)]);
    }
    
    let mut item_id: i64 = 0;
    let mut actual_points: i64 = 0;
    
    // Handle item attachment
    if item_slot >= 1 && item_slot <= 9 {
        if let Ok(Some(inventory)) = db::get_inventory(&server.db, char_id).await {
            let items = inventory.items();
            let slot_item = items[(item_slot - 1) as usize];
            
            if slot_item > 0 {
                item_id = slot_item as i64;
                // Remove item from sender's inventory
                if let Err(e) = db::update_item_slot(&server.db, char_id, item_slot, 0).await {
                    warn!("Failed to remove item for mail: {}", e);
                    return Ok(vec![build_mail_send_response(false)]);
                }
            }
        }
    }
    
    // Handle points attachment
    if points > 0 {
        let current_points = session.read().await.points;
        if points > current_points {
            warn!("Mail send failed: insufficient points (has {} wants to send {})", current_points, points);
            return Ok(vec![build_mail_send_response(false)]);
        }
        
        let new_points = current_points - points;
        
        // Update sender's points
        if let Err(e) = db::update_points(&server.db, char_id, new_points as i64).await {
            warn!("Failed to deduct points for mail: {}", e);
            return Ok(vec![build_mail_send_response(false)]);
        }
        
        // Update session
        {
            let mut session_guard = session.write().await;
            session_guard.points = new_points;
        }
        
        actual_points = points as i64;
    }
    
    // Send the mail
    match db::send_mail(&server.db, char_id, receiver.id, &sender_name, &message, item_id, actual_points).await {
        Ok(mail_id) => {
            info!("Mail {} sent from {} to {}", mail_id, sender_name, receiver_name);
            
            // TODO: If receiver is online, notify them of new mail
            
            // Build response with updated points
            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::MailSend.id())
                .write_u8(1) // success
                .write_u32(session.read().await.points);
            
            // If we removed an item, also send item slot update
            let mut responses = vec![writer.into_bytes()];
            
            if item_id > 0 {
                let mut item_writer = MessageWriter::new();
                item_writer.write_u16(MessageType::ReturnItem.id())
                    .write_u8(item_slot)
                    .write_u16(0); // item removed
                responses.push(item_writer.into_bytes());
            }
            
            Ok(responses)
        }
        Err(e) => {
            warn!("Failed to send mail: {}", e);
            Ok(vec![build_mail_send_response(false)])
        }
    }
}

/// Build mail send response
fn build_mail_send_response(success: bool) -> Vec<u8> {
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::MailSend.id())
        .write_u8(if success { 1 } else { 0 });
    writer.into_bytes()
}

/// Handle MSG_MAIL_RECEIVER_CHECK (80)
/// Check if a username exists (for mail UI validation)
/// 
/// Client sends:
/// - username (string)
/// 
/// Server responds:
/// - exists (u8): 1 = exists, 0 = doesn't exist
pub async fn handle_mail_receiver_check(
    payload: &[u8],
    server: &Arc<Server>,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    
    let username = reader.read_string()?;
    
    // Check if user exists
    let exists = match db::find_character_by_username(&server.db, &username).await {
        Ok(Some(_)) => true,
        _ => false,
    };
    
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::MailReceiverCheck.id())
        .write_u8(if exists { 1 } else { 0 });
    
    debug!("Mail receiver check for '{}': {}", username, if exists { "exists" } else { "not found" });
    
    Ok(vec![writer.into_bytes()])
}
