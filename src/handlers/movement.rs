//! Movement message handlers

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageWriter, MovementUpdate};
use crate::Server;

/// Handle movement message
pub async fn handle_movement(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
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
