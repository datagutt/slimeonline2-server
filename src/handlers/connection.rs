//! Connection handling for client connections

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use crate::constants::*;
use crate::crypto::{decrypt_client_message, encrypt_server_message};
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageWriter, peek_message_type, write_player_left};
use crate::Server;

use super::auth;

/// Handle a client connection.
pub async fn handle_connection(
    mut socket: TcpStream,
    addr: SocketAddr,
    server: Arc<Server>,
) -> Result<()> {
    let ip = addr.ip().to_string();
    info!("New connection from {}", addr);

    // Track IP connection
    server.add_ip_connection(&ip);

    // Create session
    let session = Arc::new(RwLock::new(PlayerSession::new(ip.clone())));
    let session_id = session.read().await.session_id;
    server.sessions.insert(session_id, session.clone());

    // Handle connection result
    let result = handle_client_messages(&mut socket, &addr, &server, session.clone()).await;

    // Cleanup on disconnect
    cleanup_session(&server, session.clone(), &mut socket).await;
    server.sessions.remove(&session_id);
    server.remove_ip_connection(&ip);

    info!("Connection closed from {}", addr);
    result
}

/// Main message loop for a client.
/// 
/// Message format from 39dll (format=0, the default):
/// - 2 bytes: payload length (little-endian u16) - NOT encrypted
/// - N bytes: encrypted payload (RC4 encrypted with CLIENT_ENCRYPT_KEY)
/// 
/// The payload itself contains:
/// - 2 bytes: message type (little-endian u16)
/// - Variable: message-specific data
async fn handle_client_messages(
    socket: &mut TcpStream,
    addr: &SocketAddr,
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<()> {
    let mut recv_buffer = BytesMut::with_capacity(MAX_MESSAGE_SIZE);
    let read_timeout = Duration::from_secs(CONNECTION_TIMEOUT_SECS);

    loop {
        // Check for timeout
        {
            let session_guard = session.read().await;
            if session_guard.is_timed_out() {
                warn!("Connection timed out for {}", addr);
                return Ok(());
            }
        }

        // Read data with timeout
        let mut temp_buf = [0u8; 4096];
        let read_result = timeout(read_timeout, socket.read(&mut temp_buf)).await;

        match read_result {
            Ok(Ok(0)) => {
                // Connection closed
                debug!("Client {} disconnected", addr);
                return Ok(());
            }
            Ok(Ok(n)) => {
                // Add raw bytes to buffer (we'll decrypt per-message, not per-read)
                recv_buffer.extend_from_slice(&temp_buf[..n]);
                debug!("Received {} bytes, buffer now has {} bytes", n, recv_buffer.len());

                // Update activity
                session.write().await.update_activity();

                // Process all complete messages in buffer
                // Format: [2-byte length][encrypted payload]
                while recv_buffer.len() >= 2 {
                    // Read the 2-byte length prefix (NOT encrypted)
                    let payload_len = u16::from_le_bytes([recv_buffer[0], recv_buffer[1]]) as usize;
                    
                    debug!("Payload length from header: {} bytes", payload_len);
                    
                    // Sanity check on length
                    if payload_len > MAX_MESSAGE_SIZE {
                        error!("Invalid payload length {} from {}", payload_len, addr);
                        return Ok(());
                    }
                    
                    // Check if we have the complete message
                    if recv_buffer.len() < 2 + payload_len {
                        debug!("Waiting for more data: have {}, need {}", recv_buffer.len(), 2 + payload_len);
                        break;
                    }
                    
                    // Extract the length prefix
                    let _ = recv_buffer.split_to(2);
                    
                    // Extract the encrypted payload
                    let mut payload = recv_buffer.split_to(payload_len).to_vec();
                    
                    debug!("Raw encrypted payload ({} bytes): {:02X?}", payload.len(), &payload[..std::cmp::min(payload.len(), 32)]);
                    
                    // Decrypt the payload
                    decrypt_client_message(&mut payload);
                    
                    debug!("Decrypted payload: {:02X?}", &payload[..std::cmp::min(payload.len(), 32)]);
                    
                    // Parse message type (first 2 bytes of decrypted payload)
                    if payload.len() < 2 {
                        warn!("Payload too short after decryption");
                        continue;
                    }
                    
                    let msg_type = u16::from_le_bytes([payload[0], payload[1]]);
                    debug!("Message type: {} (0x{:04X})", msg_type, msg_type);
                    
                    // Handle the message (payload includes message type)
                    let responses = handle_message(
                        msg_type,
                        &payload[2..], // Skip message type bytes
                        server,
                        session.clone(),
                    ).await?;

                    // Send all responses
                    for response in responses {
                        send_message(socket, response).await?;
                    }

                    // Check if we should disconnect (logout)
                    if msg_type == MSG_LOGOUT {
                        return Ok(());
                    }
                }
            }
            Ok(Err(e)) => {
                error!("Read error from {}: {}", addr, e);
                return Err(e.into());
            }
            Err(_) => {
                // Read timeout - just continue and check timeout flag
                continue;
            }
        }
    }
}

/// Try to parse a complete message from the buffer.
/// Returns (message_type, total_message_length) if complete message available.
fn try_parse_message(buffer: &BytesMut) -> Option<(u16, usize)> {
    if buffer.len() < 2 {
        return None;
    }

    let msg_type = peek_message_type(buffer)?;

    // Determine message length based on type
    match msg_type {
        // Fixed-length messages (just the type, 2 bytes)
        MSG_PING | MSG_LOGOUT | MSG_CANMOVE_TRUE | MSG_PLAYER_STOP => {
            Some((msg_type, 2))
        }

        // Login/Register: version + username + password + mac (4 strings)
        MSG_LOGIN | MSG_REGISTER => {
            find_strings_end(buffer, 2, 4).map(|len| (msg_type, len))
        }

        // Chat: just one string
        MSG_CHAT => {
            find_strings_end(buffer, 2, 1).map(|len| (msg_type, len))
        }

        // Typing indicator: no payload after type
        MSG_PLAYER_TYPING => Some((msg_type, 2)),

        // Movement: direction (1) + optional coordinates
        MSG_MOVE_PLAYER => {
            if buffer.len() < 3 {
                return None;
            }
            let direction = buffer[2];
            let len = match direction {
                DIR_START_LEFT_GROUND | DIR_START_RIGHT_GROUND |
                DIR_STOP_LEFT_GROUND | DIR_STOP_RIGHT_GROUND |
                DIR_LANDING => 2 + 1 + 2 + 2, // type + dir + x + y
                DIR_JUMP => 2 + 1 + 2,         // type + dir + x
                _ => 2 + 1,                     // type + dir only
            };
            if buffer.len() >= len {
                Some((msg_type, len))
            } else {
                None
            }
        }

        // Response to NEW_PLAYER: target_pid (2) + our_x (2) + our_y (2)
        MSG_NEW_PLAYER => {
            if buffer.len() >= 8 {
                Some((msg_type, 8))
            } else {
                None
            }
        }

        // Use item: slot (1 byte)
        MSG_USE_ITEM => {
            if buffer.len() >= 3 {
                Some((msg_type, 3))
            } else {
                None
            }
        }

        // Discard item: slot (1 byte)  
        MSG_DISCARD_ITEM => {
            if buffer.len() >= 3 {
                Some((msg_type, 3))
            } else {
                None
            }
        }

        // Take discarded item: instance_id (2 bytes)
        MSG_DISCARDED_ITEM_TAKE => {
            if buffer.len() >= 4 {
                Some((msg_type, 4))
            } else {
                None
            }
        }

        // Change outfit/accessory: slot (1 byte)
        MSG_CHANGE_OUT | MSG_CHANGE_ACS1 | MSG_CHANGE_ACS2 => {
            if buffer.len() >= 3 {
                Some((msg_type, 3))
            } else {
                None
            }
        }

        // Emote: emote_id (1 byte)
        MSG_EMOTE => {
            if buffer.len() >= 3 {
                Some((msg_type, 3))
            } else {
                None
            }
        }

        // Action: action_id (1 byte)
        MSG_ACTION => {
            if buffer.len() >= 3 {
                Some((msg_type, 3))
            } else {
                None
            }
        }

        // Bank process: operation (1) + amount (4) + optional receiver string
        MSG_BANK_PROCESS => {
            if buffer.len() < 7 {
                return None;
            }
            let operation = buffer[2];
            if operation == 3 {
                // Transfer - has receiver string
                find_strings_end(buffer, 7, 1).map(|len| (msg_type, len))
            } else {
                Some((msg_type, 7))
            }
        }

        // Shop buy: category (1) + item_id (2)
        MSG_SHOP_BUY => {
            if buffer.len() >= 5 {
                Some((msg_type, 5))
            } else {
                None
            }
        }

        // Sell request prices: no payload
        MSG_SELL_REQ_PRICES => Some((msg_type, 2)),

        // Sell: slot (1)
        MSG_SELL => {
            if buffer.len() >= 3 {
                Some((msg_type, 3))
            } else {
                None
            }
        }

        // For other messages, we need to handle them case by case
        // Default: log and skip 2 bytes (message type only)
        _ => {
            debug!("Unknown message type {}, treating as 2-byte message", msg_type);
            Some((msg_type, 2))
        }
    }
}

/// Find the end position after n null-terminated strings starting at offset.
fn find_strings_end(buffer: &BytesMut, start: usize, count: usize) -> Option<usize> {
    let mut pos = start;
    for _ in 0..count {
        // Find next null terminator
        while pos < buffer.len() && buffer[pos] != 0 {
            pos += 1;
        }
        if pos >= buffer.len() {
            return None; // String not terminated
        }
        pos += 1; // Skip null terminator
    }
    Some(pos)
}

/// Handle a single message. Returns a list of response messages to send.
async fn handle_message(
    msg_type: u16,
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    debug!("Handling message type {}", msg_type);

    match msg_type {
        MSG_PING => {
            let mut writer = MessageWriter::new();
            crate::protocol::write_ping(&mut writer);
            Ok(vec![writer.into_bytes()])
        }

        MSG_LOGIN => {
            auth::handle_login(payload, server, session).await
        }

        MSG_REGISTER => {
            auth::handle_register(payload, server, session).await
        }

        MSG_LOGOUT => {
            // Session cleanup is handled in the main loop
            Ok(vec![])
        }

        MSG_MOVE_PLAYER => {
            handle_movement(payload, server, session).await
        }

        MSG_CHAT => {
            handle_chat(payload, server, session).await
        }

        MSG_PLAYER_TYPING => {
            handle_typing(server, session).await
        }

        MSG_NEW_PLAYER => {
            // Response to new player notification - client sending their position
            handle_new_player_response(payload, server, session).await
        }

        MSG_EMOTE => {
            handle_emote(payload, server, session).await
        }

        MSG_ACTION => {
            handle_action(payload, server, session).await
        }

        MSG_CHANGE_OUT => {
            handle_change_outfit(payload, server, session).await
        }

        MSG_CHANGE_ACS1 => {
            handle_change_accessory1(payload, server, session).await
        }

        MSG_CHANGE_ACS2 => {
            handle_change_accessory2(payload, server, session).await
        }

        _ => {
            debug!("Unhandled message type: {}", msg_type);
            Ok(vec![])
        }
    }
}

/// Handle movement message
async fn handle_movement(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    use crate::protocol::MovementUpdate;

    let mut reader = MessageReader::new(payload);
    let movement = MovementUpdate::parse(&mut reader)?;

    let (player_id, room_id, should_broadcast) = {
        let mut session_guard = session.write().await;
        
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        let player_id = match session_guard.player_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        // Update position if provided
        if let Some(x) = movement.x {
            session_guard.x = x;
        }
        if let Some(y) = movement.y {
            session_guard.y = y;
        }

        (player_id, session_guard.room_id, true)
    };

    if !should_broadcast {
        return Ok(vec![]);
    }

    // Broadcast movement to other players in the room
    let room_players = server.game_state.get_room_players(room_id).await;
    
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue; // Don't send to self
        }

        // Find the other player's session and queue the message
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                // Build movement broadcast
                let mut writer = MessageWriter::new();
                movement.write_broadcast(&mut writer, player_id);
                
                // Queue message for the other player
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle chat message
async fn handle_chat(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    use crate::protocol::ChatMessage;

    let mut reader = MessageReader::new(payload);
    let chat = ChatMessage::parse(&mut reader)?;

    // Validate message length
    if chat.message.len() > MAX_CHAT_LENGTH {
        return Ok(vec![]);
    }

    let (player_id, room_id, username) = {
        let session_guard = session.read().await;
        
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        let player_id = match session_guard.player_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        (
            player_id,
            session_guard.room_id,
            session_guard.username.clone().unwrap_or_default(),
        )
    };

    info!("[CHAT] {}: {}", username, chat.message);

    // Broadcast to all players in room (including sender)
    let room_players = server.game_state.get_room_players(room_id).await;
    
    for other_player_id in room_players {
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                ChatMessage::write_broadcast(&mut writer, player_id, &chat.message);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle typing indicator
async fn handle_typing(
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let (player_id, room_id) = {
        let session_guard = session.read().await;
        
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        match session_guard.player_id {
            Some(id) => (id, session_guard.room_id),
            None => return Ok(vec![]),
        }
    };

    // Broadcast typing indicator to other players in room
    let room_players = server.game_state.get_room_players(room_id).await;
    
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                writer.write_u16(MSG_PLAYER_TYPING).write_u16(player_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle new player response (client sending their position)
async fn handle_new_player_response(
    payload: &[u8],
    _server: &Arc<Server>,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.len() < 6 {
        return Ok(vec![]);
    }

    let mut reader = MessageReader::new(payload);
    let _target_pid = reader.read_u16()?;
    let _our_x = reader.read_u16()?;
    let _our_y = reader.read_u16()?;

    // This is just acknowledgment from client, no response needed
    Ok(vec![])
}

/// Handle emote
async fn handle_emote(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let emote_id = payload[0];

    let (player_id, room_id) = {
        let session_guard = session.read().await;
        
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        match session_guard.player_id {
            Some(id) => (id, session_guard.room_id),
            None => return Ok(vec![]),
        }
    };

    // Broadcast emote to all players in room
    let room_players = server.game_state.get_room_players(room_id).await;
    
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                writer.write_u16(MSG_EMOTE).write_u16(player_id).write_u8(emote_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle action (sit, etc.)
async fn handle_action(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let action_id = payload[0];

    let (player_id, room_id) = {
        let session_guard = session.read().await;
        
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        match session_guard.player_id {
            Some(id) => (id, session_guard.room_id),
            None => return Ok(vec![]),
        }
    };

    // Broadcast action to all players in room
    let room_players = server.game_state.get_room_players(room_id).await;
    
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                writer.write_u16(MSG_ACTION).write_u16(player_id).write_u8(action_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle outfit change
async fn handle_change_outfit(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let slot = payload[0];

    let (player_id, room_id, new_body_id) = {
        let mut session_guard = session.write().await;
        
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        let player_id = match session_guard.player_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        // Update body_id in session (would need inventory lookup for actual ID)
        // For now, just use slot as a placeholder
        session_guard.body_id = slot as u16;

        (player_id, session_guard.room_id, session_guard.body_id)
    };

    // Broadcast outfit change to all players in room
    let room_players = server.game_state.get_room_players(room_id).await;
    
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                writer.write_u16(MSG_CHANGE_OUT).write_u16(player_id).write_u16(new_body_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle accessory 1 change
async fn handle_change_accessory1(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let slot = payload[0];

    let (player_id, room_id, new_acs_id) = {
        let mut session_guard = session.write().await;
        
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        let player_id = match session_guard.player_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        session_guard.acs1_id = slot as u16;

        (player_id, session_guard.room_id, session_guard.acs1_id)
    };

    // Broadcast accessory change to all players in room
    let room_players = server.game_state.get_room_players(room_id).await;
    
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                writer.write_u16(MSG_CHANGE_ACS1).write_u16(player_id).write_u16(new_acs_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle accessory 2 change
async fn handle_change_accessory2(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let slot = payload[0];

    let (player_id, room_id, new_acs_id) = {
        let mut session_guard = session.write().await;
        
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        let player_id = match session_guard.player_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        session_guard.acs2_id = slot as u16;

        (player_id, session_guard.room_id, session_guard.acs2_id)
    };

    // Broadcast accessory change to all players in room
    let room_players = server.game_state.get_room_players(room_id).await;
    
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                writer.write_u16(MSG_CHANGE_ACS2).write_u16(player_id).write_u16(new_acs_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Send a message to the client.
/// 
/// Message format for 39dll (format=0):
/// - 2 bytes: payload length (little-endian u16) - NOT encrypted
/// - N bytes: encrypted payload
pub async fn send_message(socket: &mut TcpStream, mut data: Vec<u8>) -> Result<()> {
    // Encrypt the payload
    encrypt_server_message(&mut data);
    
    // Build the complete message with length prefix
    let len = data.len() as u16;
    let mut message = Vec::with_capacity(2 + data.len());
    message.extend_from_slice(&len.to_le_bytes()); // 2-byte length prefix (NOT encrypted)
    message.extend_from_slice(&data);              // Encrypted payload
    
    socket.write_all(&message).await?;
    socket.flush().await?;
    Ok(())
}

/// Clean up session on disconnect.
async fn cleanup_session(
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
    socket: &mut TcpStream,
) {
    let (player_id, room_id, character_id, username) = {
        let session_guard = session.read().await;
        (
            session_guard.player_id,
            session_guard.room_id,
            session_guard.character_id,
            session_guard.username.clone(),
        )
    };

    if let Some(player_id) = player_id {
        // Get current position for saving
        let (x, y, points) = {
            let session_guard = session.read().await;
            (session_guard.x, session_guard.y, session_guard.points)
        };

        // Save player data to database
        if let Some(char_id) = character_id {
            if let Err(e) = crate::db::update_position(
                &server.db,
                char_id,
                x as i16,
                y as i16,
                room_id as i16,
            ).await {
                error!("Failed to save position for character {}: {}", char_id, e);
            }

            if let Err(e) = crate::db::update_points(&server.db, char_id, points as i64).await {
                error!("Failed to save points for character {}: {}", char_id, e);
            }
        }

        // Broadcast player left to room
        let room_players = server.game_state.get_room_players(room_id).await;
        for other_player_id in room_players {
            if other_player_id == player_id {
                continue;
            }

            if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
                if let Some(other_session) = server.sessions.get(&other_session_id) {
                    let mut writer = MessageWriter::new();
                    write_player_left(&mut writer, player_id);
                    other_session.write().await.queue_message(writer.into_bytes());
                }
            }
        }

        // Remove from room and active players
        server.game_state.remove_player_from_room(player_id, room_id).await;
        server.active_player_ids.remove(&player_id);

        if let Some(name) = username {
            info!("Player {} (ID: {}) logged out", name, player_id);
        }
    }
}
