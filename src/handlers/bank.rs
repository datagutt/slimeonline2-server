//! Bank system handlers - deposit, withdraw, transfer

use std::sync::Arc;
use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageWriter, MessageType};
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
    
    let character_id = {
        let session_guard = session.read().await;
        session_guard.character_id
    };
    
    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };
    
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

/// Handle deposit: move points from wallet to bank
async fn handle_deposit(
    reader: &mut MessageReader<'_>,
    server: &Arc<Server>,
    char_id: i64,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let amount = reader.read_u32()?;
    
    if amount == 0 {
        return Ok(vec![]);
    }
    
    // Get current balances
    let current_points = session.read().await.points;
    let current_bank = db::get_bank_balance(&server.db, char_id).await.unwrap_or(0);
    
    // Validate: player has enough points
    if amount > current_points {
        warn!("Deposit failed: player {} tried to deposit {} but only has {} points", 
              char_id, amount, current_points);
        return Ok(vec![]);
    }
    
    // Validate: bank won't exceed max
    let new_bank = current_bank + amount as i64;
    if new_bank > MAX_BANK_BALANCE as i64 {
        warn!("Deposit failed: would exceed max bank balance");
        return Ok(vec![]);
    }
    
    let new_points = current_points - amount;
    
    // Update database
    if let Err(e) = db::update_points_and_bank(&server.db, char_id, new_points as i64, new_bank).await {
        warn!("Failed to update bank: {}", e);
        return Ok(vec![]);
    }
    
    // Update session
    {
        let mut session_guard = session.write().await;
        session_guard.points = new_points;
    }
    
    debug!("Deposit OK: {} points -> bank. New points: {}, New bank: {}", amount, new_points, new_bank);
    
    // Send response: case 1 + new_points + new_bank
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BankProcess.id())
        .write_u8(1)
        .write_u32(new_points)
        .write_u32(new_bank as u32);
    
    Ok(vec![writer.into_bytes()])
}

/// Handle withdraw: move points from bank to wallet
async fn handle_withdraw(
    reader: &mut MessageReader<'_>,
    server: &Arc<Server>,
    char_id: i64,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let amount = reader.read_u32()?;
    
    if amount == 0 {
        return Ok(vec![]);
    }
    
    // Get current balances
    let current_points = session.read().await.points;
    let current_bank = db::get_bank_balance(&server.db, char_id).await.unwrap_or(0);
    
    // Validate: bank has enough
    if (amount as i64) > current_bank {
        warn!("Withdraw failed: player {} tried to withdraw {} but only has {} in bank", 
              char_id, amount, current_bank);
        return Ok(vec![]);
    }
    
    // Validate: wallet won't exceed max (client checks sl_points <= 1000000)
    let new_points = current_points.saturating_add(amount);
    if new_points > MAX_POINTS {
        warn!("Withdraw failed: would exceed max points");
        return Ok(vec![]);
    }
    
    let new_bank = current_bank - amount as i64;
    
    // Update database
    if let Err(e) = db::update_points_and_bank(&server.db, char_id, new_points as i64, new_bank).await {
        warn!("Failed to update bank: {}", e);
        return Ok(vec![]);
    }
    
    // Update session
    {
        let mut session_guard = session.write().await;
        session_guard.points = new_points;
    }
    
    debug!("Withdraw OK: {} points <- bank. New points: {}, New bank: {}", amount, new_points, new_bank);
    
    // Send response: case 2 + new_points + new_bank
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BankProcess.id())
        .write_u8(2)
        .write_u32(new_points)
        .write_u32(new_bank as u32);
    
    Ok(vec![writer.into_bytes()])
}

/// Handle transfer: send money from bank to another player's bank
async fn handle_transfer(
    reader: &mut MessageReader<'_>,
    server: &Arc<Server>,
    char_id: i64,
) -> Result<Vec<Vec<u8>>> {
    let receiver_name = reader.read_string()?;
    let amount = reader.read_u32()?;
    
    if amount == 0 || receiver_name.is_empty() {
        return Ok(vec![]);
    }
    
    // Get sender's bank balance
    let sender_bank = db::get_bank_balance(&server.db, char_id).await.unwrap_or(0);
    
    // Validate: sender has enough
    if (amount as i64) > sender_bank {
        warn!("Transfer failed: player {} tried to transfer {} but only has {} in bank", 
              char_id, amount, sender_bank);
        return Ok(vec![]);
    }
    
    // Find receiver
    let receiver = match db::find_character_by_username(&server.db, &receiver_name).await {
        Ok(Some(char)) => char,
        Ok(None) => {
            debug!("Transfer failed: receiver '{}' not found", receiver_name);
            // Send case 4: receiver not found
            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::BankProcess.id())
                .write_u8(4);
            return Ok(vec![writer.into_bytes()]);
        }
        Err(e) => {
            warn!("Transfer failed: database error: {}", e);
            return Ok(vec![]);
        }
    };
    
    // Don't allow transfer to self
    if receiver.id == char_id {
        warn!("Transfer failed: player {} tried to transfer to self", char_id);
        return Ok(vec![]);
    }
    
    // Check receiver's bank won't exceed max
    let receiver_new_bank = receiver.bank_balance + amount as i64;
    if receiver_new_bank > MAX_BANK_BALANCE as i64 {
        warn!("Transfer failed: receiver bank would exceed max");
        return Ok(vec![]);
    }
    
    let sender_new_bank = sender_bank - amount as i64;
    
    // Execute transfer atomically using a transaction
    match db::transfer_bank_funds(&server.db, char_id, sender_new_bank, receiver.id, receiver_new_bank).await {
        Ok(()) => {
            debug!("Transfer OK: {} points from {} to {}. Sender new bank: {}", 
                   amount, char_id, receiver_name, sender_new_bank);
            
            // Send response: case 3 + new_bank
            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::BankProcess.id())
                .write_u8(3)
                .write_u32(sender_new_bank as u32);
            
            Ok(vec![writer.into_bytes()])
        }
        Err(e) => {
            warn!("Transfer failed: transaction error: {}", e);
            Ok(vec![])
        }
    }
}
