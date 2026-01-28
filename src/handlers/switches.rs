//! Switch/trigger system handlers
//!
//! Handles MSG_SWITCH_SET (116) for co-op puzzles and interactive switches.
//!
//! Protocol:
//! - Client → Server: MSG_SWITCH_SET + switch_id (u8)
//! - Server → Clients: MSG_SWITCH_SET + room_id (u16) + switch_id (u8) + status (u8)
//!
//! Status values:
//! - 0 = switch off (no players)
//! - 1 = switch on (1 player)
//! - 2+ = special trigger (multiple players for co-op puzzles)

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::debug;

use crate::Server;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};

/// Handle MSG_SWITCH_SET from client
///
/// When a player steps on a switch, the client sends:
/// - switch_id (u8): The ID of the switch being activated
///
/// The server tracks how many players have activated each switch and broadcasts
/// the status to all players in the room.
pub async fn handle_switch_set(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let mut reader = MessageReader::new(payload);
    let switch_id = reader.read_u8()?;

    let (player_id, room_id) = {
        let session_guard = session.read().await;
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }
        (session_guard.player_id, session_guard.room_id)
    };

    let player_id = match player_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Get or create room and activate switch
    let room = server.game_state.get_or_create_room(room_id);
    let new_status = room.activate_switch(switch_id, player_id).await;

    // Only broadcast if this is a new activation
    if let Some(status) = new_status {
        debug!(
            "Player {} activated switch {} in room {}, new status: {}",
            player_id, switch_id, room_id, status
        );

        // Broadcast to all players in the room
        broadcast_switch_status(server, room_id, switch_id, status).await;
    }

    Ok(vec![])
}

/// Broadcast switch status to all players in a room
pub async fn broadcast_switch_status(server: &Server, room_id: u16, switch_id: u8, status: u8) {
    let room_players = server.game_state.get_room_players(room_id).await;

    // Build the message: MSG_SWITCH_SET + room_id + switch_id + status
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::SwitchSet.id())
        .write_u16(room_id)
        .write_u8(switch_id)
        .write_u8(status);
    let msg = writer.into_bytes();

    for player_id in room_players {
        if let Some(session_id) = server.game_state.players_by_id.get(&player_id)
            && let Some(handle) = server.sessions.get(&session_id)
        {
            handle.queue_message(msg.clone()).await;
        }
    }
}

/// Handle player leaving a room - deactivate their switches and broadcast updates
///
/// This should be called when a player warps or disconnects.
pub async fn handle_player_leave_room(server: &Server, player_id: u16, room_id: u16) {
    if let Some(room) = server.game_state.get_room(room_id) {
        let affected_switches = room.deactivate_player_switches(player_id).await;

        // Broadcast status updates for affected switches
        for (switch_id, new_status) in affected_switches {
            debug!(
                "Player {} left room {}, switch {} now has status {}",
                player_id, room_id, switch_id, new_status
            );
            broadcast_switch_status(server, room_id, switch_id, new_status).await;
        }

        // If room is now empty, reset all switches
        let player_count = room.player_count().await;
        if player_count == 0 {
            room.reset_switches().await;
        }
    }
}
