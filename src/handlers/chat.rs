//! Chat, emote, and action message handlers

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::info;

use crate::constants::*;
use crate::game::PlayerSession;
use crate::protocol::{ChatMessage, MessageReader, MessageWriter};
use crate::Server;

/// Handle chat message
pub async fn handle_chat(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
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
                writer.write_u16(MSG_PLAYER_TYPING).write_u16(player_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle emote
pub async fn handle_emote(
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
pub async fn handle_action(
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
