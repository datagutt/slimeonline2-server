//! Gameplay handlers (points collection, items, etc.)

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::debug;

use crate::constants::MAX_POINTS;
use crate::game::PlayerSession;
use crate::protocol::{MessageType, MessageWriter};
use crate::Server;

/// Handle point collection (slime points scattered on maps)
///
/// Client sends: MSG_POINT (18) + point_index (1 byte)
/// Server should:
/// 1. Increment player's points
/// 2. Broadcast to other players that this point was taken
pub async fn handle_point_collection(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let point_index = payload[0];

    let (player_id, room_id, _new_points) = {
        let mut session_guard = session.write().await;

        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        let player_id = match session_guard.player_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        // Increment points (cap at MAX_POINTS)
        if session_guard.points < MAX_POINTS {
            session_guard.points += 1;
        }

        debug!(
            "Player {} collected point {} (total: {})",
            player_id, point_index, session_guard.points
        );

        (player_id, session_guard.room_id, session_guard.points)
    };

    // Broadcast to other players in the room that this point was taken
    let room_players = server.game_state.get_room_players(room_id).await;

    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                // Tell other clients this point was taken
                writer
                    .write_u16(MessageType::Point.id())
                    .write_u8(point_index);
                other_session
                    .write()
                    .await
                    .queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}
