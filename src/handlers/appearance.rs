//! Appearance change handlers (outfit, accessories)

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use crate::game::PlayerSession;
use crate::protocol::{MessageWriter, MessageType};
use crate::Server;

/// Handle outfit change
pub async fn handle_change_outfit(
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
                writer.write_u16(MessageType::ChangeOutfit.id()).write_u16(player_id).write_u16(new_body_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle accessory 1 change
pub async fn handle_change_accessory1(
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
                writer.write_u16(MessageType::ChangeAccessory1.id()).write_u16(player_id).write_u16(new_acs_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle accessory 2 change
pub async fn handle_change_accessory2(
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
                writer.write_u16(MessageType::ChangeAccessory2.id()).write_u16(player_id).write_u16(new_acs_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}
