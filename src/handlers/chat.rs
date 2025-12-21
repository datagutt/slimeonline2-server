//! Chat, emote, and action message handlers

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::constants::MAX_CHAT_LENGTH;
use crate::game::PlayerSession;
use crate::protocol::{ChatMessage, MessageReader, MessageWriter, MessageType};
use crate::rate_limit::ActionType;
use crate::validation::{sanitize_chat, validate_chat_message};
use crate::Server;

/// Handle chat message
pub async fn handle_chat(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let chat = ChatMessage::parse(&mut reader)?;

    // Get session info for rate limiting
    let (player_id, room_id, session_id, username) = {
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
            session_guard.session_id,
            session_guard.username.clone().unwrap_or_default(),
        )
    };

    // Rate limit chat messages
    let rate_result = server.rate_limiter.check_player(
        session_id.as_u128() as u64,
        ActionType::Chat,
    ).await;

    if !rate_result.is_allowed() {
        warn!("Chat rate limited for player {}", username);
        return Ok(vec![]);
    }

    // Validate and sanitize message
    let message = match validate_chat_message(&chat.message) {
        Ok(msg) => msg.to_string(),
        Err(e) => {
            warn!("Invalid chat message from {}: {}", username, e.message);
            // Sanitize and use anyway if it's just too long
            if chat.message.len() > MAX_CHAT_LENGTH {
                sanitize_chat(&chat.message)
            } else {
                return Ok(vec![]);
            }
        }
    };

    if message.is_empty() {
        return Ok(vec![]);
    }

    info!("[CHAT] {}: {}", username, message);

    // Broadcast to all players in room (including sender)
    let room_players = server.game_state.get_room_players(room_id).await;
    
    for other_player_id in room_players {
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                ChatMessage::write_broadcast(&mut writer, player_id, &message);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle typing indicator
pub async fn handle_typing(
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
                writer.write_u16(MessageType::PlayerTyping.id()).write_u16(player_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Dice emote ID - server generates random result 1-6
const DICE_EMOTE_ID: u8 = 13;

/// Handle emote
pub async fn handle_emote(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    use rand::Rng;
    
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let mut emote_id = payload[0];

    // Validate emote ID (there are only a handful of valid emotes)
    if emote_id > 20 {
        return Ok(vec![]);
    }

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

    // Special handling for dice emote (id 13)
    // Server generates random result 1-6, sends as emote_id 14-19
    // (Client displays different dice faces based on emote_id)
    if emote_id == DICE_EMOTE_ID {
        let mut rng = rand::thread_rng();
        let dice_roll: u8 = rng.gen_range(1..=6);
        // Dice results are mapped to emote IDs 14-19 (14=1, 15=2, ..., 19=6)
        emote_id = 13 + dice_roll;
    }

    // Broadcast emote to all players in room (including sender for dice)
    let room_players = server.game_state.get_room_players(room_id).await;
    
    for other_player_id in room_players {
        // For dice, we broadcast to everyone including the sender
        // so they see the server-generated result
        if other_player_id == player_id && emote_id < 14 {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                writer.write_u16(MessageType::Emote.id()).write_u16(player_id).write_u8(emote_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle action (sit, etc.)
pub async fn handle_action(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let action_id = payload[0];

    // Validate action ID
    if action_id > 10 {
        return Ok(vec![]);
    }

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
                writer.write_u16(MessageType::Action.id()).write_u16(player_id).write_u8(action_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle new player response (client sending their position)
pub async fn handle_new_player_response(
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
