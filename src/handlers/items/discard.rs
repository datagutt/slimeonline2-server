//! MSG_DISCARD_ITEM (39) handler
//!
//! Client sends: slot (1 byte) + x (2 bytes) + y (2 bytes)
//! Server broadcasts: x (2) + y (2) + item_id (2) + instance_id (2)
//!
//! Also provides helper to send dropped items when player enters a room.

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::anticheat::validate_position_bounds;
use crate::constants::ITEM_SLOTS;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

use super::database::can_discard_item;

/// Handle MSG_DISCARD_ITEM (39)
pub async fn handle_discard_item(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.len() < 5 {
        return Ok(vec![]);
    }

    let mut reader = MessageReader::new(payload);
    let slot = reader.read_u8()?;
    let drop_x = reader.read_u16()?;
    let drop_y = reader.read_u16()?;

    // Validate slot
    if slot < 1 || slot > ITEM_SLOTS as u8 {
        warn!("Invalid discard slot: {}", slot);
        return Ok(vec![]);
    }

    // Validate drop position
    if !validate_position_bounds(drop_x, drop_y) {
        warn!("Invalid discard position: ({}, {})", drop_x, drop_y);
        return Ok(vec![]);
    }

    let (character_id, player_id, room_id) = {
        let session_guard = session.read().await;
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }
        (
            session_guard.character_id,
            session_guard.player_id,
            session_guard.room_id,
        )
    };

    let character_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    let player_id = match player_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Get the item in this slot from database
    let inventory = match crate::db::get_inventory(&server.db, character_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };

    let items = inventory.items();
    let item_id = items[(slot - 1) as usize];

    if item_id == 0 {
        debug!("Slot {} is empty, nothing to discard", slot);
        return Ok(vec![]);
    }

    // Check if item can be discarded
    if !can_discard_item(item_id) {
        debug!("Item {} cannot be discarded", item_id);
        return Ok(vec![]);
    }

    info!(
        "Player {} discarding item {} from slot {} at ({}, {}) in room {}",
        player_id, item_id, slot, drop_x, drop_y, room_id
    );

    // Remove item from inventory
    crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;

    // Save to database with 3 minute expiration (180 seconds) like original server
    let db_id = crate::db::add_ground_item(
        &server.db,
        room_id,
        item_id,
        drop_x,
        drop_y,
        Some(character_id),
        Some(180), // 3 minutes expiration
    )
    .await?;

    // Use DB id as instance_id (cast to u16, will wrap for very large values but that's fine)
    let instance_id = db_id as u16;

    // Broadcast to all players in the room
    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_handle) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                writer
                    .write_u16(MessageType::DiscardItem.id())
                    .write_u16(drop_x)
                    .write_u16(drop_y)
                    .write_u16(item_id)
                    .write_u16(instance_id);
                other_handle.queue_message(writer.into_bytes()).await;
            }
        }
    }

    Ok(vec![])
}

/// Build messages for all dropped items in a room.
/// Called when a player enters a room to show them existing dropped items.
///
/// From original server (room_check_discarded_item.gml):
/// For each obj_discarded_item in the room, send MSG_DISCARD_ITEM with x, y, item_id, _id
///
/// Items are loaded from the database for persistence across server restarts.
/// The instance_id sent to client is the DB row id (cast to u16).
pub async fn write_room_dropped_items(server: &Arc<Server>, room_id: u16) -> Vec<Vec<u8>> {
    // Load dropped items from database
    let dropped_items = match crate::db::get_ground_items(&server.db, room_id).await {
        Ok(items) => items,
        Err(e) => {
            warn!("Failed to load ground items for room {}: {}", room_id, e);
            return Vec::new();
        }
    };

    let mut messages = Vec::new();
    for item in dropped_items {
        let mut writer = MessageWriter::new();
        // Use DB id as instance_id (cast to u16)
        let instance_id = item.id as u16;
        writer
            .write_u16(MessageType::DiscardItem.id())
            .write_u16(item.x as u16)
            .write_u16(item.y as u16)
            .write_u16(item.item_id as u16)
            .write_u16(instance_id);
        messages.push(writer.into_bytes());
    }

    messages
}
