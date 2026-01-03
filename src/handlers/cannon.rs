//! Cannon system handlers
//!
//! Handles cannon-related messages (broadcast relay system):
//! - MSG_CANNON_ENTER (98) - Player enters cannon
//! - MSG_CANNON_MOVE (99) - Player rotates cannon
//! - MSG_CANNON_SET_POWER (100) - Player sets cannon power
//! - MSG_CANNON_SHOOT (101) - Player shoots from cannon

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::debug;

use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

/// Handle MSG_CANNON_ENTER (98) - Player enters a cannon
pub async fn handle_cannon_enter(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let cannon_id = reader.read_u8()?;
    let angle = reader.read_u8()?;

    let (player_id, room_id) = {
        let session_guard = session.read().await;
        (session_guard.player_id, session_guard.room_id)
    };

    let player_id = match player_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    debug!(
        "Cannon enter: player={}, cannon={}, angle={}",
        player_id, cannon_id, angle
    );

    // Broadcast to all players in same room
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::CannonEnter.id());
    writer.write_u16(player_id);
    writer.write_u8(cannon_id);
    writer.write_u8(angle);
    let msg = writer.into_bytes();

    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(other_session_id.value()) {
                other_session.queue_message(msg.clone()).await;
            }
        }
    }

    Ok(vec![])
}

/// Handle MSG_CANNON_MOVE (99) - Player rotates cannon
pub async fn handle_cannon_move(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let cannon_id = reader.read_u8()?;
    let angle = reader.read_u8()?;
    let direction = reader.read_u8()?; // 1 = left, 2 = right

    let (player_id, room_id) = {
        let session_guard = session.read().await;
        (session_guard.player_id, session_guard.room_id)
    };

    let player_id = match player_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    debug!(
        "Cannon move: cannon={}, angle={}, dir={}",
        cannon_id, angle, direction
    );

    // Broadcast to all players in same room (no player_id in this message)
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::CannonMove.id());
    writer.write_u8(cannon_id);
    writer.write_u8(angle);
    writer.write_u8(direction);
    let msg = writer.into_bytes();

    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(other_session_id.value()) {
                other_session.queue_message(msg.clone()).await;
            }
        }
    }

    Ok(vec![])
}

/// Handle MSG_CANNON_SET_POWER (100) - Player sets cannon power
pub async fn handle_cannon_set_power(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let cannon_id = reader.read_u8()?;
    let power = reader.read_u8()?;

    let (player_id, room_id) = {
        let session_guard = session.read().await;
        (session_guard.player_id, session_guard.room_id)
    };

    let player_id = match player_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    debug!("Cannon set power: cannon={}, power={}", cannon_id, power);

    // Broadcast to all players in same room (no player_id in this message)
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::CannonSetPower.id());
    writer.write_u8(cannon_id);
    writer.write_u8(power);
    let msg = writer.into_bytes();

    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(other_session_id.value()) {
                other_session.queue_message(msg.clone()).await;
            }
        }
    }

    Ok(vec![])
}

/// Handle MSG_CANNON_SHOOT (101) - Player shoots from cannon
pub async fn handle_cannon_shoot(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let cannon_id = reader.read_u8()?;
    let angle = reader.read_u8()?;
    let power = reader.read_u8()?;
    let x = reader.read_u16()?;
    let y = reader.read_u16()?;

    let (player_id, room_id) = {
        let session_guard = session.read().await;
        (session_guard.player_id, session_guard.room_id)
    };

    let player_id = match player_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    debug!(
        "Cannon shoot: player={}, cannon={}, angle={}, power={}, pos=({},{})",
        player_id, cannon_id, angle, power, x, y
    );

    // Update player position on server
    {
        let mut session_guard = session.write().await;
        session_guard.x = x;
        session_guard.y = y;
    }

    // Broadcast to all players in same room
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::CannonShoot.id());
    writer.write_u8(cannon_id);
    writer.write_u8(angle);
    writer.write_u8(power);
    writer.write_u16(player_id);
    writer.write_u16(x);
    writer.write_u16(y);
    let msg = writer.into_bytes();

    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(other_session_id.value()) {
                other_session.queue_message(msg.clone()).await;
            }
        }
    }

    Ok(vec![])
}
