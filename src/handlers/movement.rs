//! Movement message handlers

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::warn;

use crate::anticheat::{validate_position_bounds, CheatResult};
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageWriter, MovementUpdate};
use crate::rate_limit::ActionType;
use crate::Server;

/// Handle movement message
pub async fn handle_movement(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let movement = MovementUpdate::parse(&mut reader)?;

    let (player_id, room_id, session_id, old_x, old_y) = {
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
            session_guard.x,
            session_guard.y,
        )
    };

    // Rate limiting for movement (very lenient)
    if !server
        .rate_limiter
        .check_player(session_id.as_u128() as u64, ActionType::Movement)
        .await
        .is_allowed()
    {
        // Just silently drop excessive movement packets
        return Ok(vec![]);
    }

    // Validate position bounds if coordinates are provided
    if let (Some(x), Some(y)) = (movement.x, movement.y) {
        if !validate_position_bounds(x, y) {
            warn!("Player {} sent invalid position: ({}, {})", player_id, x, y);
            return Ok(vec![]);
        }

        // Anti-cheat: check for teleportation
        let cheat_result = server
            .anticheat
            .check_movement(session_id.as_u128() as u64, x, y, room_id)
            .await;

        match cheat_result {
            CheatResult::Clean => {
                // Update position
                let mut session_guard = session.write().await;
                session_guard.x = x;
                session_guard.y = y;
            }
            CheatResult::Suspicious { reason, severity } => {
                // Log but allow - could be lag
                if severity >= 2 {
                    warn!(
                        "Suspicious movement from player {}: {} (was: {},{} now: {},{})",
                        player_id, reason, old_x, old_y, x, y
                    );
                }
                // Still update position to avoid desyncs
                let mut session_guard = session.write().await;
                session_guard.x = x;
                session_guard.y = y;
            }
            CheatResult::Cheating { reason } => {
                warn!("Cheat detected for player {}: {}", player_id, reason);

                // Check if player should be kicked (repeated violations)
                if server
                    .anticheat
                    .should_kick(session_id.as_u128() as u64)
                    .await
                {
                    warn!("Kicking player {} for repeated movement cheats", player_id);
                    let mut session_guard = session.write().await;
                    session_guard.kick(format!("Movement cheat detected: {}", reason));
                }

                // Don't update position or broadcast this movement
                return Ok(vec![]);
            }
        }
    } else {
        // Direction-only update (no position) - just update flags
        // These are valid and don't need position checks
    }

    // Broadcast movement to other players in the room
    let room_players = server.game_state.get_room_players(room_id).await;

    for other_player_id in room_players {
        if other_player_id == player_id {
            continue; // Don't send to self
        }

        // Find the other player's session and queue the message
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_handle) = server.sessions.get(&other_session_id) {
                // Build movement broadcast
                let mut writer = MessageWriter::new();
                movement.write_broadcast(&mut writer, player_id);

                // Queue message for the other player
                other_handle.queue_message(writer.into_bytes()).await;
            }
        }
    }

    Ok(vec![])
}
