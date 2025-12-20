//! MSG_DISCARDED_ITEM_TAKE (40) handler
//!
//! Client sends: instance_id (2 bytes)
//! Server responds with MSG_GET_ITEM if successful

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

/// Handle MSG_DISCARDED_ITEM_TAKE (40)
pub async fn handle_take_dropped_item(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.len() < 2 {
        return Ok(vec![]);
    }

    let mut reader = MessageReader::new(payload);
    let instance_id = reader.read_u16()?;

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

    // Try to take the dropped item
    let dropped_item = match server
        .game_state
        .take_dropped_item(room_id, instance_id)
        .await
    {
        Some(item) => item,
        None => {
            debug!("Dropped item {} not found or already taken", instance_id);
            return Ok(vec![]);
        }
    };

    // Find an empty slot in inventory
    let inventory = match crate::db::get_inventory(&server.db, character_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };

    let items = inventory.items();
    let empty_slot = items.iter().position(|&id| id == 0);

    let slot = match empty_slot {
        Some(idx) => (idx + 1) as u8, // Slots are 1-indexed
        None => {
            // Inventory full - put the item back
            server
                .game_state
                .add_dropped_item_with_id(
                    room_id,
                    dropped_item.x,
                    dropped_item.y,
                    dropped_item.item_id,
                    instance_id,
                )
                .await;
            debug!("Inventory full, cannot pick up item");
            return Ok(vec![]);
        }
    };

    info!(
        "Player {:?} picking up item {} (instance {}) into slot {}",
        player_id, dropped_item.item_id, instance_id, slot
    );

    // Add item to inventory
    crate::db::update_item_slot(&server.db, character_id, slot, dropped_item.item_id as i16)
        .await?;

    // Send MSG_GET_ITEM to player
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::GetItem.id())
        .write_u8(slot)
        .write_u16(dropped_item.item_id);

    Ok(vec![writer.into_bytes()])
}
