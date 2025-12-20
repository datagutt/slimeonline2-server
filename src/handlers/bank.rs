//! Bank system handlers - deposit, withdraw, transfer

use std::sync::Arc;
use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageWriter, MessageType};
use crate::rate_limit::ActionType;
use crate::Server;
use crate::db;
use crate::constants::{MAX_POINTS, MAX_BANK_BALANCE};

/// Handle MSG_REQUEST_STATUS (44)
/// Client requests status of a game element (bank balance, etc.)
pub async fn handle_request_status(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    
    let element = reader.read_u8()?;
    
    match element {
        1 => {
            // Bank status request
            let character_id = session.read().await.character_id;
            
            if let Some(char_id) = character_id {
                let bank_balance = db::get_bank_balance(&server.db, char_id).await.unwrap_or(0);
                
                let mut writer = MessageWriter::new();
                writer.write_u16(MessageType::RequestStatus.id())
                    .write_u8(1)  // element = bank
                    .write_u32(bank_balance as u32);
                
                debug!("Sending bank balance: {}", bank_balance);
                return Ok(vec![writer.into_bytes()]);
            }
        }
        _ => {
            debug!("Unknown RequestStatus element: {}", element);
        }
    }
    
    Ok(vec![])
}

/// Handle MSG_BANK_PROCESS (45)
/// Deposit, withdraw, or transfer money
pub async fn handle_bank_process(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    
    let case = reader.read_u8()?;
    
    let (character_id, session_id) = {
        let session_guard = session.read().await;
        (session_guard.character_id, session_guard.session_id)
    };
    
    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Rate limit bank transactions
    if !server.rate_limiter.check_player(session_id.as_u128() as u64, ActionType::Bank)
        .await
        .is_allowed()
    {
        debug!("Bank transaction rate limited for character {}", char_id);
        return Ok(vec![]);
    }
    
    match case {
        1 => handle_deposit(&mut reader, server, char_id, session).await,
        2 => handle_withdraw(&mut reader, server, char_id, session).await,
        3 => handle_transfer(&mut reader, server, char_id).await,
        _ => {
            warn!("Unknown bank process case: {}", case);
            Ok(vec![])
        }
    }
}

/// Helper to build a deposit response (case 1)
fn build_deposit_response(points: u32, bank: i64) -> Vec<u8> {
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BankProcess.id())
        .write_u8(1)
        .write_u32(points)
        .write_u32(bank as u32);
    writer.into_bytes()
}

/// Helper to build a withdraw response (case 2)
fn build_withdraw_response(points: u32, bank: i64) -> Vec<u8> {
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BankProcess.id())
        .write_u8(2)
        .write_u32(points)
        .write_u32(bank as u32);
    writer.into_bytes()
}

/// Helper to build a transfer response (case 3)
fn build_transfer_response(bank: i64) -> Vec<u8> {
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BankProcess.id())
        .write_u8(3)
        .write_u32(bank as u32);
    writer.into_bytes()
}

/// Helper to build a "receiver not found" response (case 4)
fn build_receiver_not_found_response() -> Vec<u8> {
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BankProcess.id())
        .write_u8(4);
    writer.into_bytes()
}

/// Handle deposit: move points from wallet to bank
async fn handle_deposit(
    reader: &mut MessageReader<'_>,
    server: &Arc<Server>,
    char_id: i64,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let amount = reader.read_u32()?;
    
    // Get current balances
    let current_points = session.read().await.points;
    let current_bank = db::get_bank_balance(&server.db, char_id).await.unwrap_or(0);
    
    // Validate: amount must be positive
    if amount == 0 {
        // Send current values back to reset UI
        return Ok(vec![build_deposit_response(current_points, current_bank)]);
    }
    
    // Validate: player has enough points
    if amount > current_points {
        warn!("Deposit failed: player {} tried to deposit {} but only has {} points", 
              char_id, amount, current_points);
        // Send current values back to reset UI
        return Ok(vec![build_deposit_response(current_points, current_bank)]);
    }
    
    // Validate: bank won't exceed max
    let new_bank = current_bank + amount as i64;
    if new_bank > MAX_BANK_BALANCE as i64 {
        warn!("Deposit failed: would exceed max bank balance");
        // Send current values back to reset UI
        return Ok(vec![build_deposit_response(current_points, current_bank)]);
    }
    
    let new_points = current_points - amount;
    
    // Update database
    if let Err(e) = db::update_points_and_bank(&server.db, char_id, new_points as i64, new_bank).await {
        warn!("Failed to update bank: {}", e);
        // Send current values back to reset UI
        return Ok(vec![build_deposit_response(current_points, current_bank)]);
    }
    
    // Update session
    {
        let mut session_guard = session.write().await;
        session_guard.points = new_points;
    }
    
    debug!("Deposit OK: {} points -> bank. New points: {}, New bank: {}", amount, new_points, new_bank);
    
    Ok(vec![build_deposit_response(new_points, new_bank)])
}

/// Handle withdraw: move points from bank to wallet
async fn handle_withdraw(
    reader: &mut MessageReader<'_>,
    server: &Arc<Server>,
    char_id: i64,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let amount = reader.read_u32()?;
    
    // Get current balances
    let current_points = session.read().await.points;
    let current_bank = db::get_bank_balance(&server.db, char_id).await.unwrap_or(0);
    
    // Validate: amount must be positive
    if amount == 0 {
        // Send current values back to reset UI
        return Ok(vec![build_withdraw_response(current_points, current_bank)]);
    }
    
    // Validate: bank has enough
    if (amount as i64) > current_bank {
        warn!("Withdraw failed: player {} tried to withdraw {} but only has {} in bank", 
              char_id, amount, current_bank);
        // Send current values back to reset UI
        return Ok(vec![build_withdraw_response(current_points, current_bank)]);
    }
    
    // Validate: wallet won't exceed max (client checks sl_points <= 1000000)
    let new_points = current_points.saturating_add(amount);
    if new_points > MAX_POINTS {
        warn!("Withdraw failed: would exceed max points");
        // Send current values back to reset UI
        return Ok(vec![build_withdraw_response(current_points, current_bank)]);
    }
    
    let new_bank = current_bank - amount as i64;
    
    // Update database
    if let Err(e) = db::update_points_and_bank(&server.db, char_id, new_points as i64, new_bank).await {
        warn!("Failed to update bank: {}", e);
        // Send current values back to reset UI
        return Ok(vec![build_withdraw_response(current_points, current_bank)]);
    }
    
    // Update session
    {
        let mut session_guard = session.write().await;
        session_guard.points = new_points;
    }
    
    debug!("Withdraw OK: {} points <- bank. New points: {}, New bank: {}", amount, new_points, new_bank);
    
    Ok(vec![build_withdraw_response(new_points, new_bank)])
}

/// Handle transfer: send money from bank to another player's bank
async fn handle_transfer(
    reader: &mut MessageReader<'_>,
    server: &Arc<Server>,
    char_id: i64,
) -> Result<Vec<Vec<u8>>> {
    let receiver_name = reader.read_string()?;
    let amount = reader.read_u32()?;
    
    // Get sender's bank balance
    let sender_bank = db::get_bank_balance(&server.db, char_id).await.unwrap_or(0);
    
    // Validate: amount must be positive and receiver name not empty
    if amount == 0 || receiver_name.is_empty() {
        // Send current balance back to reset UI
        return Ok(vec![build_transfer_response(sender_bank)]);
    }
    
    // Validate: sender has enough
    if (amount as i64) > sender_bank {
        warn!("Transfer failed: player {} tried to transfer {} but only has {} in bank", 
              char_id, amount, sender_bank);
        // Send current balance back to reset UI
        return Ok(vec![build_transfer_response(sender_bank)]);
    }
    
    // Find receiver
    let receiver = match db::find_character_by_username(&server.db, &receiver_name).await {
        Ok(Some(char)) => char,
        Ok(None) => {
            debug!("Transfer failed: receiver '{}' not found", receiver_name);
            // Send case 4: receiver not found (special UI handling)
            return Ok(vec![build_receiver_not_found_response()]);
        }
        Err(e) => {
            warn!("Transfer failed: database error: {}", e);
            // Send current balance back to reset UI
            return Ok(vec![build_transfer_response(sender_bank)]);
        }
    };
    
    // Don't allow transfer to self
    if receiver.id == char_id {
        warn!("Transfer failed: player {} tried to transfer to self", char_id);
        // Send current balance back to reset UI
        return Ok(vec![build_transfer_response(sender_bank)]);
    }
    
    // Check receiver's bank won't exceed max
    let receiver_new_bank = receiver.bank_balance + amount as i64;
    if receiver_new_bank > MAX_BANK_BALANCE as i64 {
        warn!("Transfer failed: receiver bank would exceed max");
        // Send current balance back to reset UI
        return Ok(vec![build_transfer_response(sender_bank)]);
    }
    
    let sender_new_bank = sender_bank - amount as i64;
    
    // Execute transfer atomically using a transaction
    match db::transfer_bank_funds(&server.db, char_id, sender_new_bank, receiver.id, receiver_new_bank).await {
        Ok(()) => {
            debug!("Transfer OK: {} points from {} to {}. Sender new bank: {}", 
                   amount, char_id, receiver_name, sender_new_bank);
            
            Ok(vec![build_transfer_response(sender_new_bank)])
        }
        Err(e) => {
            warn!("Transfer failed: transaction error: {}", e);
            // Send current balance back to reset UI
            Ok(vec![build_transfer_response(sender_bank)])
        }
    }
}
