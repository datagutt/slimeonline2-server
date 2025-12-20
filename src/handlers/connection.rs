//! Connection handling for client connections

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tracing::{debug, error, info, trace, warn};

use crate::constants::*;
use crate::crypto::{decrypt_client_message, encrypt_server_message};
use crate::game::PlayerSession;
use crate::protocol::{MessageWriter, write_player_left, MessageType, describe_message};
use crate::Server;

use super::{auth, movement, chat, appearance, gameplay, warp, items, shop, bank};

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

/// Server-initiated ping interval (20 seconds, client times out at 30)
const PING_INTERVAL: Duration = Duration::from_secs(20);

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
    
    // Create ping interval timer - client expects server to send pings to keep alive
    let mut ping_interval = tokio::time::interval(PING_INTERVAL);
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    
    // Skip the first immediate tick
    ping_interval.tick().await;

    loop {
        // Check for timeout
        {
            let session_guard = session.read().await;
            if session_guard.is_timed_out() {
                warn!("Connection timed out for {}", addr);
                return Ok(());
            }
        }

        // Use select! to handle both incoming data and ping timer
        let mut temp_buf = [0u8; 4096];
        
        tokio::select! {
            // Ping timer fired - send ping to client and any queued messages
            _ = ping_interval.tick() => {
                // Only send pings to authenticated clients
                let is_authenticated = session.read().await.is_authenticated;
                if is_authenticated {
                    debug!("Sending keepalive ping to {}", addr);
                    let mut writer = MessageWriter::new();
                    crate::protocol::write_ping(&mut writer);
                    if let Err(e) = send_message(socket, writer.into_bytes()).await {
                        error!("Failed to send ping to {}: {}", addr, e);
                        return Err(e);
                    }
                }
                
                // Also flush any queued messages (from broadcasts by other players, etc.)
                let queued_messages = session.write().await.drain_messages();
                for msg in queued_messages {
                    if let Err(e) = send_message(socket, msg).await {
                        error!("Failed to send queued message to {}: {}", addr, e);
                        return Err(e);
                    }
                }
            }
            
            // Data available to read
            read_result = socket.read(&mut temp_buf) => {
                match read_result {
                    Ok(0) => {
                        // Connection closed
                        debug!("Client {} disconnected", addr);
                        return Ok(());
                    }
                    Ok(n) => {
                        // Add raw bytes to buffer (we'll decrypt per-message, not per-read)
                        recv_buffer.extend_from_slice(&temp_buf[..n]);
                        trace!("Received {} bytes, buffer now has {} bytes", n, recv_buffer.len());

                        // Update activity
                        session.write().await.update_activity();

                        // Process all complete messages in buffer
                        // Format: [2-byte length][encrypted payload]
                        while recv_buffer.len() >= 2 {
                            // Read the 2-byte length prefix (NOT encrypted)
                            let payload_len = u16::from_le_bytes([recv_buffer[0], recv_buffer[1]]) as usize;
                            
                            trace!("Payload length from header: {} bytes", payload_len);
                            
                            // Sanity check on length
                            if payload_len > MAX_MESSAGE_SIZE {
                                error!("Invalid payload length {} from {}", payload_len, addr);
                                return Ok(());
                            }
                            
                            // Check if we have the complete message
                            if recv_buffer.len() < 2 + payload_len {
                                trace!("Waiting for more data: have {}, need {}", recv_buffer.len(), 2 + payload_len);
                                break;
                            }
                            
                            // Extract the length prefix
                            let _ = recv_buffer.split_to(2);
                            
                            // Extract the encrypted payload
                            let mut payload = recv_buffer.split_to(payload_len).to_vec();
                            
                            trace!("Raw encrypted payload ({} bytes): {:02X?}", payload.len(), &payload[..std::cmp::min(payload.len(), 32)]);
                            
                            // Decrypt the payload
                            decrypt_client_message(&mut payload);
                            
                            trace!("Decrypted payload: {:02X?}", &payload[..std::cmp::min(payload.len(), 32)]);
                            
                            // Parse message type (first 2 bytes of decrypted payload)
                            if payload.len() < 2 {
                                warn!("Payload too short after decryption");
                                continue;
                            }
                            
                            let msg_type_id = u16::from_le_bytes([payload[0], payload[1]]);
                            let msg_type = MessageType::from_id(msg_type_id);
                            
                            // Log message with parsed fields (skip high-frequency messages)
                            if !msg_type.is_high_frequency() {
                                let description = describe_message(msg_type_id, &payload[2..]);
                                debug!("{}", description);
                            } else {
                                trace!("Message: {}", msg_type);
                            }
                            
                            // Handle the message (payload includes message type)
                            let responses = handle_message(
                                msg_type,
                                &payload[2..], // Skip message type bytes
                                server,
                                session.clone(),
                            ).await?;

                            // Send all direct responses from the handler
                            for response in responses {
                                send_message(socket, response).await?;
                            }

                            // Also send any queued messages (from broadcasts, etc.)
                            let queued_messages = session.write().await.drain_messages();
                            for msg in queued_messages {
                                send_message(socket, msg).await?;
                            }

                            // Check if we should disconnect (logout)
                            if matches!(msg_type, MessageType::Logout) {
                                return Ok(());
                            }
                        }
                    }
                    Err(e) => {
                        error!("Read error from {}: {}", addr, e);
                        return Err(e.into());
                    }
                }
            }
        }
    }
}

/// Handle a single message. Returns a list of response messages to send.
async fn handle_message(
    msg_type: MessageType,
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    match msg_type {
        MessageType::Ping => {
            // Client sent a ping - respond with ping (keepalive acknowledgment)
            let mut writer = MessageWriter::new();
            crate::protocol::write_ping(&mut writer);
            Ok(vec![writer.into_bytes()])
        }

        MessageType::Login => {
            auth::handle_login(payload, server, session).await
        }

        MessageType::Register => {
            auth::handle_register(payload, server, session).await
        }

        MessageType::Logout => {
            // Session cleanup is handled in the main loop
            Ok(vec![])
        }

        MessageType::MovePlayer => {
            movement::handle_movement(payload, server, session).await
        }

        MessageType::Chat => {
            chat::handle_chat(payload, server, session).await
        }

        MessageType::PlayerTyping => {
            chat::handle_typing(server, session).await
        }

        MessageType::NewPlayer => {
            chat::handle_new_player_response(payload, server, session).await
        }

        MessageType::Emote => {
            chat::handle_emote(payload, server, session).await
        }

        MessageType::Action => {
            chat::handle_action(payload, server, session).await
        }

        MessageType::ChangeOutfit => {
            appearance::handle_change_outfit(payload, server, session).await
        }

        MessageType::ChangeAccessory1 => {
            appearance::handle_change_accessory1(payload, server, session).await
        }

        MessageType::ChangeAccessory2 => {
            appearance::handle_change_accessory2(payload, server, session).await
        }

        MessageType::Point => {
            gameplay::handle_point_collection(payload, server, session).await
        }

        MessageType::Warp => {
            warp::handle_warp(payload, server, session).await
        }

        MessageType::UseItem => {
            items::handle_use_item(payload, server, session).await
        }

        MessageType::DiscardItem => {
            items::handle_discard_item(payload, server, session).await
        }

        MessageType::DiscardedItemTake => {
            items::handle_take_dropped_item(payload, server, session).await
        }

        MessageType::ShopBuy => {
            shop::handle_shop_buy(payload, server, session).await
        }

        MessageType::RequestStatus => {
            bank::handle_request_status(payload, server, session).await
        }

        MessageType::BankProcess => {
            bank::handle_bank_process(payload, server, session).await
        }

        MessageType::PingReq => {
            // Client is requesting latency measurement - echo back PingReq
            let mut writer = MessageWriter::new();
            crate::protocol::write_ping_req(&mut writer);
            Ok(vec![writer.into_bytes()])
        }

        MessageType::PlayerStop => {
            // Client stopped moving (interacting with NPC/bank/etc)
            // Format: x (u16) + y (u16)
            // Broadcast to other players in room
            let mut reader = crate::protocol::MessageReader::new(payload);
            if let (Ok(x), Ok(y)) = (reader.read_u16(), reader.read_u16()) {
                let (player_id, room_id) = {
                    let session_guard = session.read().await;
                    (session_guard.player_id, session_guard.room_id)
                };

                if let Some(player_id) = player_id {
                    // Update session position
                    {
                        let mut session_guard = session.write().await;
                        session_guard.x = x;
                        session_guard.y = y;
                    }

                    // Broadcast to other players in room
                    let room_players = server.game_state.get_room_players(room_id).await;
                    for other_player_id in room_players {
                        if other_player_id == player_id {
                            continue;
                        }
                        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
                            if let Some(other_session) = server.sessions.get(&other_session_id) {
                                let mut writer = MessageWriter::new();
                                writer.write_u16(MessageType::PlayerStop.id())
                                    .write_u16(player_id)
                                    .write_u16(x)
                                    .write_u16(y);
                                other_session.write().await.queue_message(writer.into_bytes());
                            }
                        }
                    }
                }
            }
            Ok(vec![])
        }

        MessageType::ToolEquip => {
            // Client equipped a tool
            // Format: slot (u8)
            let mut reader = crate::protocol::MessageReader::new(payload);
            if let Ok(slot) = reader.read_u8() {
                // Validate slot range
                if slot < 1 || slot > 9 {
                    warn!("Invalid tool slot: {}", slot);
                    return Ok(vec![]);
                }

                let character_id = session.read().await.character_id;
                if let Some(char_id) = character_id {
                    // Verify player owns a tool in this slot
                    if let Ok(Some(inventory)) = crate::db::get_inventory(&server.db, char_id).await {
                        let tools = inventory.tools();
                        let tool_id = tools[(slot - 1) as usize];
                        
                        if tool_id == 0 {
                            warn!("Player {} tried to equip empty tool slot {}", char_id, slot);
                            return Ok(vec![]);
                        }

                        if let Err(e) = crate::db::update_equipped_tool(&server.db, char_id, slot as i16).await {
                            error!("Failed to save equipped tool: {}", e);
                        }
                    }
                }
            }
            Ok(vec![])
        }

        MessageType::ToolUnequip => {
            // Client unequipped their tool
            let character_id = session.read().await.character_id;
            if let Some(char_id) = character_id {
                if let Err(e) = crate::db::update_equipped_tool(&server.db, char_id, 0).await {
                    error!("Failed to clear equipped tool: {}", e);
                }
            }
            Ok(vec![])
        }

        _ => {
            debug!("Unhandled: [{}] {}", msg_type.category(), msg_type);
            Ok(vec![])
        }
    }
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
    _socket: &mut TcpStream,
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
