//! Music changer handlers
//!
//! Handles music changer in city rooms:
//! - MSG_MUSIC_CHANGER_LIST (95) - Get available music tracks
//! - MSG_MUSIC_CHANGER_SET (96) - Change room music

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::db;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

/// Music changer cooldown in seconds (4 hours)
const MUSIC_COOLDOWN_SECS: i64 = 4 * 60 * 60;

/// Handle MSG_MUSIC_CHANGER_LIST (95) - Get available music tracks
pub async fn handle_music_changer_list(
    _payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let room_id = session.read().await.room_id;

    debug!("Music changer list request: room={}", room_id);

    // Check if music changer is available in this room
    let music_state = match db::get_music_changer_state(&server.db, room_id).await? {
        Some(state) => state,
        None => {
            warn!("Music changer not available in room {}", room_id);
            return Ok(vec![]);
        }
    };

    // Check if on cooldown
    if music_state.is_on_cooldown() {
        let mut writer = MessageWriter::new();
        writer.write_u16(MessageType::MusicChangerList.id());
        writer.write_u8(0); // Cannot change (on cooldown)
        return Ok(vec![writer.into_bytes()]);
    }

    // Send available tracks
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::MusicChangerList.id());
    writer.write_u8(1); // Can change

    // Write 6 track slots (3 day, 3 night)
    writer.write_u8(music_state.day_track_1);
    writer.write_u8(music_state.day_track_2);
    writer.write_u8(if music_state.day_track_3_unlocked {
        music_state.day_track_3
    } else {
        0
    });
    writer.write_u8(music_state.night_track_1);
    writer.write_u8(music_state.night_track_2);
    writer.write_u8(if music_state.night_track_3_unlocked {
        music_state.night_track_3
    } else {
        0
    });

    Ok(vec![writer.into_bytes()])
}

/// Handle MSG_MUSIC_CHANGER_SET (96) - Change room music
pub async fn handle_music_changer_set(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let track = reader.read_u8()?; // 1-3 = day tracks, 4-6 = night tracks

    let room_id = session.read().await.room_id;

    debug!("Music changer set: room={}, track={}", room_id, track);

    // Check if music changer is available
    let music_state = match db::get_music_changer_state(&server.db, room_id).await? {
        Some(state) => state,
        None => {
            warn!("Music changer not available in room {}", room_id);
            return Ok(vec![]);
        }
    };

    // Check if on cooldown
    if music_state.is_on_cooldown() {
        let mut writer = MessageWriter::new();
        writer.write_u16(MessageType::MusicChangerSet.id());
        writer.write_u8(0); // Failed (on cooldown)
        return Ok(vec![writer.into_bytes()]);
    }

    // Validate track selection
    if track == 3 && !music_state.day_track_3_unlocked {
        warn!("Day track 3 not unlocked");
        return Ok(vec![]);
    }
    if track == 6 && !music_state.night_track_3_unlocked {
        warn!("Night track 3 not unlocked");
        return Ok(vec![]);
    }

    // Get the actual music ID for the selected track
    let new_music = match track {
        1 => music_state.day_track_1,
        2 => music_state.day_track_2,
        3 => music_state.day_track_3,
        4 => music_state.night_track_1,
        5 => music_state.night_track_2,
        6 => music_state.night_track_3,
        _ => return Ok(vec![]),
    };

    // Check if music is actually different
    let current_music = if track <= 3 {
        music_state.current_day_music
    } else {
        music_state.current_night_music
    };

    if new_music == current_music {
        // Same music, just acknowledge
        let mut writer = MessageWriter::new();
        writer.write_u16(MessageType::MusicChangerSet.id());
        writer.write_u8(1); // Success
        return Ok(vec![writer.into_bytes()]);
    }

    // Apply the change and set cooldown
    let _ = db::set_room_music(&server.db, room_id, track, new_music, MUSIC_COOLDOWN_SECS).await;

    info!(
        "Room {} music changed: track {} = music {}",
        room_id, track, new_music
    );

    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::MusicChangerSet.id());
    writer.write_u8(1); // Success

    Ok(vec![writer.into_bytes()])
}
