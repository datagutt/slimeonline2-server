//! Mail system handlers - mailbox, send mail, check receiver

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::constants::{MAX_MAIL_BODY, MAX_POINTS};
use crate::db;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

/// Handle MSG_MAILBOX (47)
/// Client requests mailbox contents (paginated)
///
/// Client sends different cases:
/// - case 1: Check for new mail (no additional params) - auto-sent on room enter
/// - case 3: Get mailbox page - page (u8)
/// - case 5: Get specific mail content - mail_slot (u8)
/// - case 2: Claim mail item attachment - mail_slot (u8)  
/// - case 4: Claim mail points attachment - mail_slot (u8)
/// - case 6: Delete mail - mail_slot (u8)
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
            // Check for new mail count (auto-sent on room enter when mailbox object exists)
            // No additional parameters
            get_new_mail_count(server, char_id).await
        }
        2 => {
            // Open mailbox - just respond with case 2 to trigger GUI creation
            // Then client will automatically request page 0
            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::Mailbox.id()).write_u8(2); // case 2 = open mailbox GUI
            Ok(vec![writer.into_bytes()])
        }
        3 => {
            // Get mailbox page
            let page = reader.read_u8()? as i64;
            get_mailbox_page(server, char_id, page).await
        }
        4 => {
            // Delete mail
            let mail_slot = reader.read_u8()? as i64;
            delete_mail(server, char_id, mail_slot).await
        }
        5 => {
            // Get specific mail content
            let mail_slot = reader.read_u8()? as i64;
            get_mail_content(server, char_id, mail_slot).await
        }
        6 => {
            // Claim mail points (take to wallet)
            let mail_slot = reader.read_u8()? as i64;
            claim_mail_points(server, char_id, mail_slot, session.clone()).await
        }
        7 => {
            // Claim mail points (send to bank)
            let mail_slot = reader.read_u8()? as i64;
            claim_mail_points_to_bank(server, char_id, mail_slot, session).await
        }
        8 => {
            // Claim mail item attachment
            let mail_slot = reader.read_u8()? as i64;
            claim_mail_item(server, char_id, mail_slot, session).await
        }
        _ => {
            debug!("Unknown mailbox case: {}", case);
            Ok(vec![])
        }
    }
}

/// Check for new mail count (case 1 response)
/// Response format: case (1), has_mail (u8): 0 = empty, 1 = has mail
async fn get_new_mail_count(server: &Arc<Server>, character_id: i64) -> Result<Vec<Vec<u8>>> {
    let count = db::get_mail_count(&server.db, character_id)
        .await
        .unwrap_or(0);

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::Mailbox.id())
        .write_u8(1) // case 1 response
        .write_u8(if count > 0 { 1 } else { 0 }); // 1 = has mail, 0 = empty

    debug!(
        "Mail count check for character {}: {} mails",
        character_id, count
    );

    Ok(vec![writer.into_bytes()])
}

/// Get a page of mails from the mailbox (case 3 response)
/// Response format: case (3), mail_count (u8), then for each mail:
///   slot (u8), sender_name (string), date (string)
async fn get_mailbox_page(
    server: &Arc<Server>,
    character_id: i64,
    page: i64,
) -> Result<Vec<Vec<u8>>> {
    // Get mails for this page (5 per page)
    let mails = db::get_mailbox(&server.db, character_id, page)
        .await
        .unwrap_or_default();

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::Mailbox.id())
        .write_u8(3) // case 3 response
        .write_u8(mails.len() as u8);

    for (i, mail) in mails.iter().enumerate() {
        let slot = (i + 1) as u8; // 1-5 for the page
        writer
            .write_u8(slot)
            .write_string(&mail.sender_name)
            .write_string(&mail.created_at); // date string
    }

    debug!("Sending mailbox page {} with {} mails", page, mails.len());

    Ok(vec![writer.into_bytes()])
}

/// Get specific mail content (case 5 response)
/// Response format: case (5), sender (string), date (string), text (string),
///   points (u16), present_cat (u8), present_id (u16), paper (u8), font (u8)
async fn get_mail_content(
    server: &Arc<Server>,
    character_id: i64,
    mail_slot: i64,
) -> Result<Vec<Vec<u8>>> {
    // mail_slot is 1-indexed position across all mails (page*5 + slot)
    // We need to calculate page and slot
    let page = (mail_slot - 1) / 5;
    let slot_in_page = ((mail_slot - 1) % 5) as usize;

    let mails = db::get_mailbox(&server.db, character_id, page)
        .await
        .unwrap_or_default();

    if slot_in_page >= mails.len() {
        debug!("Mail slot {} not found", mail_slot);
        return Ok(vec![]);
    }

    let mail = &mails[slot_in_page];

    // Mark as read
    let _ = db::mark_mail_read(&server.db, mail.id, character_id).await;

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::Mailbox.id())
        .write_u8(5) // case 5 response
        .write_string(&mail.sender_name)
        .write_string(&mail.created_at)
        .write_string(&mail.message)
        .write_u16(mail.points as u16)
        .write_u8(mail.item_cat as u8)
        .write_u16(mail.item_id as u16)
        .write_u8(mail.paper as u8)
        .write_u8(mail.font_color as u8);

    debug!(
        "Sending mail content for slot {}: item_id={}, item_cat={}, paper={}, font={}",
        mail_slot, mail.item_id, mail.item_cat, mail.paper, mail.font_color
    );

    Ok(vec![writer.into_bytes()])
}

/// Claim mail item attachment (case 2 -> response case 8)
async fn claim_mail_item(
    server: &Arc<Server>,
    character_id: i64,
    mail_slot: i64,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let page = (mail_slot - 1) / 5;
    let slot_in_page = ((mail_slot - 1) % 5) as usize;

    let mails = db::get_mailbox(&server.db, character_id, page)
        .await
        .unwrap_or_default();

    if slot_in_page >= mails.len() {
        return Ok(vec![]);
    }

    let mail = &mails[slot_in_page];

    if mail.item_id == 0 {
        return Ok(vec![]);
    }

    // Find empty item slot
    if let Ok(Some(inventory)) = db::get_inventory(&server.db, character_id).await {
        let items = inventory.items();
        if let Some(slot_idx) = items.iter().position(|&id| id == 0) {
            let slot = (slot_idx + 1) as u8;

            // Add item to inventory
            if let Err(e) =
                db::update_item_slot(&server.db, character_id, slot, mail.item_id as i16).await
            {
                warn!("Failed to add item from mail: {}", e);
                return Ok(vec![]);
            }

            // Clear item from mail
            if let Err(e) = db::clear_mail_item(&server.db, mail.id, character_id).await {
                warn!("Failed to clear mail item: {}", e);
            }

            // Send MSG_GET_ITEM to add item to inventory
            let mut item_writer = MessageWriter::new();
            item_writer
                .write_u16(MessageType::GetItem.id())
                .write_u8(slot)
                .write_u16(mail.item_id as u16);

            // Send case 8 response (return to read screen)
            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::Mailbox.id()).write_u8(8);

            debug!("Claimed item {} from mail to slot {}", mail.item_id, slot);

            return Ok(vec![item_writer.into_bytes(), writer.into_bytes()]);
        }
    }

    warn!("No empty slot for mail item");
    Ok(vec![])
}

/// Claim mail points attachment (case 4 -> response case 6 or 7)
async fn claim_mail_points(
    server: &Arc<Server>,
    character_id: i64,
    mail_slot: i64,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let page = (mail_slot - 1) / 5;
    let slot_in_page = ((mail_slot - 1) % 5) as usize;

    let mails = db::get_mailbox(&server.db, character_id, page)
        .await
        .unwrap_or_default();

    if slot_in_page >= mails.len() {
        return Ok(vec![]);
    }

    let mail = &mails[slot_in_page];

    if mail.points == 0 {
        return Ok(vec![]);
    }

    let current_points = session.read().await.points;
    let points_to_add = mail.points as u32;
    let new_points = (current_points as u64 + points_to_add as u64).min(MAX_POINTS as u64) as u32;
    let actually_added = new_points - current_points;

    // Update points in database
    if let Err(e) = db::update_points(&server.db, character_id, new_points as i64).await {
        warn!("Failed to add points from mail: {}", e);
        return Ok(vec![]);
    }

    // Update session
    {
        let mut session_guard = session.write().await;
        session_guard.points = new_points;
    }

    // Clear points from mail
    if let Err(e) = db::clear_mail_points(&server.db, mail.id, character_id).await {
        warn!("Failed to clear mail points: {}", e);
    }

    // Response case 6 = points added directly (with amount), case 7 = points to bank
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::Mailbox.id())
        .write_u8(6) // case 6 = points added
        .write_u16(actually_added as u16);

    debug!(
        "Claimed {} points from mail (added {} to wallet)",
        points_to_add, actually_added
    );

    Ok(vec![writer.into_bytes()])
}

/// Claim mail points attachment and send to bank (case 7 -> response case 7)
async fn claim_mail_points_to_bank(
    server: &Arc<Server>,
    character_id: i64,
    mail_slot: i64,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let page = (mail_slot - 1) / 5;
    let slot_in_page = ((mail_slot - 1) % 5) as usize;

    let mails = db::get_mailbox(&server.db, character_id, page)
        .await
        .unwrap_or_default();

    if slot_in_page >= mails.len() {
        return Ok(vec![]);
    }

    let mail = &mails[slot_in_page];

    if mail.points == 0 {
        return Ok(vec![]);
    }

    let points_to_add = mail.points;

    // Get current bank balance and add points
    let current_bank = db::get_bank_balance(&server.db, character_id)
        .await
        .unwrap_or(0);
    let new_bank = current_bank + points_to_add;

    // Update bank balance in database
    if let Err(e) = db::update_bank_balance(&server.db, character_id, new_bank).await {
        warn!("Failed to add points to bank from mail: {}", e);
        return Ok(vec![]);
    }

    // Clear points from mail
    if let Err(e) = db::clear_mail_points(&server.db, mail.id, character_id).await {
        warn!("Failed to clear mail points: {}", e);
    }

    // Response case 7 = points sent to bank
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::Mailbox.id()).write_u8(7); // case 7 = points to bank

    debug!(
        "Claimed {} points from mail to bank (new balance: {})",
        points_to_add, new_bank
    );

    Ok(vec![writer.into_bytes()])
}

/// Delete a mail (case 4)
async fn delete_mail(
    server: &Arc<Server>,
    character_id: i64,
    mail_slot: i64,
) -> Result<Vec<Vec<u8>>> {
    let page = (mail_slot - 1) / 5;
    let slot_in_page = ((mail_slot - 1) % 5) as usize;

    let mails = db::get_mailbox(&server.db, character_id, page)
        .await
        .unwrap_or_default();

    if slot_in_page >= mails.len() {
        return Ok(vec![]);
    }

    let mail = &mails[slot_in_page];

    if let Err(e) = db::delete_mail(&server.db, mail.id, character_id).await {
        warn!("Failed to delete mail: {}", e);
    } else {
        debug!("Deleted mail {}", mail.id);
    }

    // No response needed - client just refreshes the page
    Ok(vec![])
}

/// Handle MSG_MAIL_SEND (78)
/// Send a mail to another player
///
/// Client sends:
/// - paper (u8): paper style
/// - col_number (u8): font color (1-10)
/// - receiver (string): receiver username
/// - presentcat (u8): present category (0=none, 1=outfits, 2=items, 3=acs, 4=tools)
/// - presentid (u16): present slot number
/// - points (u16): points to attach
/// - message (string): the message text (lines joined by #)
pub async fn handle_mail_send(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);

    let paper = reader.read_u8()?;
    let font_color = reader.read_u8()?;
    let receiver_name = reader.read_string()?;
    let present_cat = reader.read_u8()?;
    let present_id = reader.read_u16()?;
    let points = reader.read_u16()? as u32;
    let message = reader.read_string()?;

    debug!("Mail send: receiver='{}', message_len={}, present_cat={}, present_id={}, points={}, paper={}, font={}", 
           receiver_name, message.len(), present_cat, present_id, points, paper, font_color);

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
    let mut item_cat: i64 = 0;
    let mut actual_points: i64 = 0;

    // Handle item attachment based on present_cat
    // present_cat: 0=none, 1=outfits, 2=items, 3=accessories, 4=tools
    // present_id: slot number (1-9)
    if present_cat > 0 && (1..=9).contains(&present_id) {
        if let Ok(Some(inventory)) = db::get_inventory(&server.db, char_id).await {
            let slot_item: u16 = match present_cat {
                1 => inventory.outfits()[(present_id - 1) as usize],
                2 => inventory.items()[(present_id - 1) as usize],
                3 => inventory.accessories()[(present_id - 1) as usize],
                4 => inventory.tools()[(present_id - 1) as usize] as u16,
                _ => 0,
            };

            if slot_item > 0 {
                item_id = slot_item as i64;
                item_cat = present_cat as i64;
                // Remove item from sender's inventory based on category
                let remove_result = match present_cat {
                    1 => db::update_outfit_slot(&server.db, char_id, present_id as u8, 0).await,
                    2 => db::update_item_slot(&server.db, char_id, present_id as u8, 0).await,
                    3 => db::update_accessory_slot(&server.db, char_id, present_id as u8, 0).await,
                    4 => db::update_tool_slot(&server.db, char_id, present_id as u8, 0).await,
                    _ => Ok(()),
                };
                if let Err(e) = remove_result {
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
            warn!(
                "Mail send failed: insufficient points (has {} wants to send {})",
                current_points, points
            );
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
    match db::send_mail(
        &server.db,
        db::SendMailParams {
            from_character_id: Some(char_id),
            to_character_id: receiver.id,
            sender_name: &sender_name,
            message: &message,
            item_id,
            item_cat,
            points: actual_points,
            paper: paper as i64,
            font_color: font_color as i64,
        },
    )
    .await
    {
        Ok(mail_id) => {
            info!(
                "Mail {} sent from {} to {}",
                mail_id, sender_name, receiver_name
            );

            // If receiver is online, notify them of new mail
            notify_new_mail(server, receiver.id).await;

            // Build response with updated points
            let mut writer = MessageWriter::new();
            writer
                .write_u16(MessageType::MailSend.id())
                .write_u8(1) // success
                .write_u32(session.read().await.points);

            // The client already removes the item locally, no need to send update
            Ok(vec![writer.into_bytes()])
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
    writer
        .write_u16(MessageType::MailSend.id())
        .write_u8(if success { 1 } else { 0 });
    writer.into_bytes()
}

/// Notify a player that they have new mail (if they're online)
/// Sends MSG_MAILBOX case 1 with has_mail = 1
async fn notify_new_mail(server: &Arc<Server>, receiver_character_id: i64) {
    // Find the receiver's session by character_id
    for session_ref in server.sessions.iter() {
        let is_receiver = {
            if let Ok(session_guard) = session_ref.value().try_read() {
                session_guard.character_id == Some(receiver_character_id)
            } else {
                false
            }
        };

        if is_receiver {
            // Build the "you have mail" notification (case 1, has_mail = 1)
            let mut writer = MessageWriter::new();
            writer
                .write_u16(MessageType::Mailbox.id())
                .write_u8(1) // case 1 = mail count check response
                .write_u8(1); // 1 = has mail

            // Queue the message for the receiver
            session_ref
                .value()
                .write()
                .await
                .queue_message(writer.into_bytes());
            debug!(
                "Notified player (char_id={}) of new mail",
                receiver_character_id
            );
            return;
        }
    }
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
    let exists = matches!(
        db::find_character_by_username(&server.db, &username).await,
        Ok(Some(_))
    );

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::MailReceiverCheck.id())
        .write_u8(if exists { 1 } else { 0 });

    debug!(
        "Mail receiver check for '{}': {}",
        username,
        if exists { "exists" } else { "not found" }
    );

    Ok(vec![writer.into_bytes()])
}
