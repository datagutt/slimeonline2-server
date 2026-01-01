//! Warp/room change handlers

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

use super::{collectibles, items, shop, upgrader};

/// Handle warp/room change
///
/// Client sends: MSG_WARP (14) + room_id (2) + x (2) + y (2)
/// Server should:
/// 1. Update player's room/position in session
/// 2. Save to database
/// 3. Broadcast to old room: player left (case 2)
/// 4. Broadcast to new room: player entered (case 1)
/// 5. Send new room's existing players to the warping player
pub async fn handle_warp(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.len() < 6 {
        return Ok(vec![]);
    }

    let mut reader = MessageReader::new(payload);
    let new_room_id = reader.read_u16()?;
    let new_x = reader.read_u16()?;
    let new_y = reader.read_u16()?;

    let (player_id, old_room_id, character_id, _body_id, _acs1_id, _acs2_id, _username) = {
        let mut session_guard = session.write().await;

        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        let player_id = match session_guard.player_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        let old_room_id = session_guard.room_id;

        // Update session with new position
        session_guard.room_id = new_room_id;
        session_guard.x = new_x;
        session_guard.y = new_y;

        info!(
            "Player {} warped from room {} to room {} at ({}, {})",
            player_id, old_room_id, new_room_id, new_x, new_y
        );

        (
            player_id,
            old_room_id,
            session_guard.character_id,
            session_guard.body_id,
            session_guard.acs1_id,
            session_guard.acs2_id,
            session_guard.username.clone(),
        )
    };

    // Save new position to database (only if auto_save_position is enabled)
    if server.config.auto_save_position {
        if let Some(char_id) = character_id {
            if let Err(e) = crate::db::update_position(
                &server.db,
                char_id,
                new_x as i16,
                new_y as i16,
                new_room_id as i16,
            )
            .await
            {
                error!("Failed to save position for character {}: {}", char_id, e);
            }
        }
    }

    // Update room tracking
    let session_id = session.read().await.session_id;

    // Remove from old room, add to new room
    server
        .game_state
        .remove_player_from_room(player_id, old_room_id)
        .await;
    server
        .game_state
        .add_player_to_room(player_id, new_room_id, session_id)
        .await;

    // Broadcast to OLD room: player left (case 2)
    if old_room_id != new_room_id {
        let old_room_players = server.game_state.get_room_players(old_room_id).await;
        for other_player_id in old_room_players {
            if other_player_id == player_id {
                continue;
            }

            if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
                if let Some(other_session) = server.sessions.get(&other_session_id) {
                    let mut writer = MessageWriter::new();
                    // MSG_WARP + player_id + case(2 = left)
                    writer
                        .write_u16(MessageType::Warp.id())
                        .write_u16(player_id)
                        .write_u8(2); // Case 2 = player leaves room
                    other_session
                        .write()
                        .await
                        .queue_message(writer.into_bytes());
                }
            }
        }
    }

    // Broadcast to NEW room: player entered (case 1)
    let new_room_players = server.game_state.get_room_players(new_room_id).await;
    let mut responses = Vec::new();

    for other_player_id in &new_room_players {
        if *other_player_id == player_id {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(other_player_id) {
            if let Some(other_session_ref) = server.sessions.get(&other_session_id) {
                // Tell existing players that this player entered
                let mut writer = MessageWriter::new();
                // MSG_WARP + player_id + case(1 = entered) + x + y
                writer
                    .write_u16(MessageType::Warp.id())
                    .write_u16(player_id)
                    .write_u8(1) // Case 1 = player enters room
                    .write_u16(new_x)
                    .write_u16(new_y);
                other_session_ref
                    .write()
                    .await
                    .queue_message(writer.into_bytes());

                // Also send the existing player's info to the warping player
                let other_session = other_session_ref.read().await;
                if other_session.is_authenticated {
                    if let Some(other_username) = &other_session.username {
                        let mut new_player_writer = MessageWriter::new();
                        new_player_writer
                            .write_u16(MessageType::NewPlayer.id())
                            .write_u16(*other_player_id)
                            .write_string(other_username)
                            .write_u16(other_session.x)
                            .write_u16(other_session.y)
                            .write_u16(other_session.body_id)
                            .write_u16(other_session.acs1_id)
                            .write_u16(other_session.acs2_id);
                        responses.push(new_player_writer.into_bytes());
                    }
                }
            }
        }
    }

    // Send shop info for the new room (if any shops exist)
    match shop::build_room_shop_info(server, new_room_id).await {
        Ok(Some(shop_msg)) => {
            responses.push(shop_msg);
        }
        Ok(None) => {
            // No shops in this room
        }
        Err(e) => {
            error!("Failed to build shop info for room {}: {}", new_room_id, e);
        }
    }

    // Send collectible info for the new room (if any collectibles exist)
    if let Some(collectible_msg) = collectibles::write_collectible_info(server, new_room_id).await {
        responses.push(collectible_msg);
    }

    // Send dropped items info for the new room (from room_check_discarded_item in original)
    let dropped_item_msgs = items::write_room_dropped_items(server, new_room_id).await;
    responses.extend(dropped_item_msgs);

    // Send top points info for city rooms (from room_check_city in original)
    // Rooms 42 (New City) and 126 (Old City Back Alley) have the top points sign
    if new_room_id == 42 || new_room_id == 126 {
        if let Some(top_points_msg) = build_top_points_message(server).await {
            responses.push(top_points_msg);
        }
    }

    // Send unlockable objects info for the new room (bubblegum machines, etc.)
    let unlockable_msgs = upgrader::send_room_unlockables(server, new_room_id).await;
    responses.extend(unlockable_msgs);

    Ok(responses)
}

/// Build MSG_GET_TOP_POINTS (73) message with the current top player
/// Returns None if there are no players or on error
async fn build_top_points_message(server: &Arc<Server>) -> Option<Vec<u8>> {
    match crate::db::get_top_points(&server.db).await {
        Ok(Some(top)) => {
            let mut writer = MessageWriter::new();
            writer
                .write_u16(MessageType::GetTopPoints.id())
                .write_string(&top.username)
                .write_u32(top.total_points as u32);
            Some(writer.into_bytes())
        }
        Ok(None) => {
            // No players in database yet
            None
        }
        Err(e) => {
            error!("Failed to get top points: {}", e);
            None
        }
    }
}
