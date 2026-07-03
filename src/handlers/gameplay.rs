//! Gameplay handlers: slime point collection and point deduction.

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::Server;
use crate::anticheat::HackType;
use crate::game::PlayerSession;
use crate::protocol::{MessageType, MessageWriter};

/// Points lost when falling into a trap pit (`POINTSLOST_LAVA` in the original
/// server's Default.config.gmx).
const POINTS_LOST_LAVA: u32 = 10;

/// Handle point collection (MSG_POINT 18, port of `case_msg_point.gml`).
///
/// Client sends the point's room-creation-order index as one byte. The original
/// server only LOGS the collection: it dedups `(index, room)` per connection,
/// increments the total, and sends nothing back. Points always respawn on room
/// re-entry, and the "taken" display on other clients is purely client-side.
/// A repeated index in the same room is a hack alert; index 0 is always
/// rejected (bug-for-bug: the first point of every room is uncollectable).
pub async fn handle_point_collection(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let point_index = payload[0];
    if point_index == 0 {
        return Ok(vec![]);
    }

    // On a duplicate, capture what the hack report needs outside the lock.
    let duplicate = {
        let mut session_guard = session.write().await;

        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        let room_id = session_guard.room_id;
        if session_guard.points_collected.insert((point_index, room_id)) {
            let max_points = server.game_config.game.limits.max_points;
            if session_guard.points < max_points {
                session_guard.points += 1;
            }
            debug!(
                "Player {:?} collected point {} in room {} (total: {})",
                session_guard.player_id, point_index, room_id, session_guard.points
            );
            None
        } else {
            Some((
                session_guard.account_id,
                session_guard.character_id,
                session_guard.ip_address.clone(),
                session_guard.player_id,
            ))
        }
    };

    if let Some((account_id, character_id, ip_address, player_id)) = duplicate {
        warn!(
            "HACK: player {:?} tried to collect point {} twice in the same room",
            player_id, point_index
        );
        if let Some(acc_id) = account_id {
            let _ = server
                .anticheat
                .report_hack(
                    &server.db,
                    acc_id,
                    character_id,
                    HackType::SuspiciousActivity,
                    "Tried to collect a Point twice in the same room!",
                    Some(&ip_address),
                    None,
                    &server.game_config.game.anticheat,
                )
                .await;
        }
    }

    Ok(vec![])
}

/// Handle MSG_POINTS_DEC (53, port of `case_msg_points_dec.gml`): the player
/// fell into a trap; deduct the fixed penalty and reply with the new total as a
/// silent MSG_POINT (case 2) so the HUD updates without the collect sound.
pub async fn handle_points_dec(
    payload: &[u8],
    _server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    // The only defined case is 1 ("player fell into a pit").
    if payload.first() != Some(&1) {
        return Ok(vec![]);
    }

    let points = {
        let mut session_guard = session.write().await;
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }
        session_guard.points = session_guard.points.saturating_sub(POINTS_LOST_LAVA);
        session_guard.points
    };

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::Point.id())
        .write_u8(2) // update the total without a sound
        .write_u32(points);
    Ok(vec![writer.into_bytes()])
}
