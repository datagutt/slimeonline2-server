//! Racing system handlers
//!
//! Handles racing-related messages:
//! - MSG_RACE_INFO (120) - Request race info/leaderboards
//! - MSG_RACE_START (121) - Start a race
//! - MSG_RACE_CHECKPOINT (122) - Hit a checkpoint
//! - MSG_RACE_END (123) - Finish a race
//! - MSG_MOVE_GET_ON (124) - Get on a moving platform
//! - MSG_MOVE_GET_OFF (125) - Get off a moving platform

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::db;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

/// Race configurations
/// Magma Dungeon race checkpoints (room IDs in order)
const MAGMA_DUNGEON_CHECKPOINTS: [u16; 14] = [
    91, 92, 94, 94, 95, 95, 97, 97, 98, 99, 100, 101, 102, 103,
];

/// Handle MSG_RACE_INFO (120) - Request race info/leaderboards
pub async fn handle_race_info(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let info_type = reader.read_u8()?;
    let race_id = reader.read_u8()?;

    debug!("Race info request: type={}, race_id={}", info_type, race_id);

    match info_type {
        1 => {
            // Request leaderboards
            let records = db::get_race_records(&server.db, race_id).await?;

            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::RaceInfo.id());
            writer.write_u8(1); // Response type

            // Write 10 single records
            for i in 0..10 {
                if let Some(record) = records.single_records.get(i) {
                    writer.write_string(&record.name);
                    writer.write_u32(record.time_ms);
                } else {
                    writer.write_string("");
                    writer.write_u32(0);
                }
            }

            // Write 10 clan records
            for i in 0..10 {
                if let Some(record) = records.clan_records.get(i) {
                    writer.write_string(&record.name);
                    writer.write_u32(record.time_ms);
                } else {
                    writer.write_string("");
                    writer.write_u32(0);
                }
            }

            Ok(vec![writer.into_bytes()])
        }
        3 => {
            // Request time limit for rewards
            let time_limit = db::get_race_time_limit(&server.db, race_id).await?;

            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::RaceInfo.id());
            writer.write_u8(3); // Response type
            writer.write_u32(time_limit);

            Ok(vec![writer.into_bytes()])
        }
        _ => Ok(vec![]),
    }
}

/// Handle MSG_RACE_START (121) - Start a race
pub async fn handle_race_start(
    payload: &[u8],
    _server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let race_id = reader.read_u8()?;

    debug!("Race start: race_id={}", race_id);

    // Initialize race state in session
    {
        let mut session_guard = session.write().await;
        session_guard.race_id = Some(race_id);
        session_guard.race_start_time = Some(std::time::Instant::now());
        session_guard.race_checkpoints = Vec::new();
    }

    // No response needed - server just stores state
    Ok(vec![])
}

/// Handle MSG_RACE_CHECKPOINT (122) - Hit a checkpoint
pub async fn handle_race_checkpoint(
    _payload: &[u8],
    _server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    // No payload - uses server-tracked cur_room
    let mut session_guard = session.write().await;

    if session_guard.race_id.is_none() {
        warn!("Received checkpoint without active race - possible hack");
        return Ok(vec![]);
    }

    // Record the current room as a checkpoint
    let room_id = session_guard.room_id;
    session_guard.race_checkpoints.push(room_id);

    debug!(
        "Race checkpoint: room={}, total={}",
        room_id,
        session_guard.race_checkpoints.len()
    );

    Ok(vec![])
}

/// Handle MSG_RACE_END (123) - Finish a race
pub async fn handle_race_end(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let race_id = reader.read_u8()?;

    let (expected_race_id, start_time, checkpoints, username, char_id) = {
        let session_guard = session.read().await;
        (
            session_guard.race_id,
            session_guard.race_start_time,
            session_guard.race_checkpoints.clone(),
            session_guard.username.clone(),
            session_guard.character_id,
        )
    };

    // Verify race ID matches
    if expected_race_id != Some(race_id) {
        warn!("Race end with wrong race_id: expected {:?}, got {}", expected_race_id, race_id);
        return Ok(vec![]);
    }

    let start_time = match start_time {
        Some(t) => t,
        None => return Ok(vec![]),
    };

    // Verify checkpoints
    let expected_checkpoints = match race_id {
        1 => &MAGMA_DUNGEON_CHECKPOINTS[..],
        _ => return Ok(vec![]),
    };

    if checkpoints.len() != expected_checkpoints.len() {
        warn!(
            "Race checkpoint count mismatch: expected {}, got {}",
            expected_checkpoints.len(),
            checkpoints.len()
        );
        // Reset race state
        let mut session_guard = session.write().await;
        session_guard.race_id = None;
        session_guard.race_start_time = None;
        session_guard.race_checkpoints.clear();
        return Ok(vec![]);
    }

    for (i, &expected) in expected_checkpoints.iter().enumerate() {
        if checkpoints.get(i) != Some(&expected) {
            warn!(
                "Race checkpoint {} mismatch: expected {}, got {:?}",
                i,
                expected,
                checkpoints.get(i)
            );
            // Reset race state
            let mut session_guard = session.write().await;
            session_guard.race_id = None;
            session_guard.race_start_time = None;
            session_guard.race_checkpoints.clear();
            return Ok(vec![]);
        }
    }

    // Calculate race time
    let race_time_ms = start_time.elapsed().as_millis() as u32;

    info!(
        "Race {} completed by {:?} in {}ms",
        race_id, username, race_time_ms
    );

    // Check if player made top 10
    if let (Some(name), Some(char_id)) = (username, char_id) {
        let _ = db::submit_race_record(&server.db, race_id, &name, race_time_ms, char_id).await;
    }

    // Reset race state
    {
        let mut session_guard = session.write().await;
        session_guard.race_id = None;
        session_guard.race_start_time = None;
        session_guard.race_checkpoints.clear();
    }

    // Send race time to player (uses MSG_RACE_INFO with type 2)
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::RaceInfo.id());
    writer.write_u8(2); // Response type = needed time
    writer.write_u32(race_time_ms);

    Ok(vec![writer.into_bytes()])
}

/// Handle MSG_MOVE_GET_ON (124) - Get on a moving platform
pub async fn handle_move_get_on(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let stand_on = reader.read_u8()?; // 0 = nothing, 1 = player, 2 = platform
    let stand_on_id = reader.read_u16()?;

    let (player_id, room_id) = {
        let session_guard = session.read().await;
        (session_guard.player_id, session_guard.room_id)
    };

    let player_id = match player_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    debug!(
        "Move get on: player={}, stand_on={}, id={}",
        player_id, stand_on, stand_on_id
    );

    // Broadcast to all players in same room
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::MoveGetOn.id());
    writer.write_u16(player_id);
    writer.write_u8(stand_on);
    writer.write_u16(stand_on_id);
    let msg = writer.into_bytes();

    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(other_session_id.value()) {
                other_session.write().await.queue_message(msg.clone());
            }
        }
    }

    Ok(vec![])
}

/// Handle MSG_MOVE_GET_OFF (125) - Get off a moving platform
pub async fn handle_move_get_off(
    _payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let (player_id, room_id) = {
        let session_guard = session.read().await;
        (session_guard.player_id, session_guard.room_id)
    };

    let player_id = match player_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    debug!("Move get off: player={}", player_id);

    // Broadcast to all players in same room
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::MoveGetOff.id());
    writer.write_u16(player_id);
    let msg = writer.into_bytes();

    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(other_session_id.value()) {
                other_session.write().await.queue_message(msg.clone());
            }
        }
    }

    Ok(vec![])
}
