//! One-time item handlers
//!
//! Handles one-time collectible items that can only be taken once per player:
//! - MSG_ONE_TIME_GET (37) - Take a one-time item

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::db;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

/// Category constants
const CAT_OUTFIT: u8 = 1;
const CAT_ITEM: u8 = 2;
const CAT_ACCESSORY: u8 = 3;

/// Handle MSG_ONE_TIME_GET (37) - Take a one-time item
pub async fn handle_one_time_take(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let real_id = reader.read_u8()?;

    let (char_id, room_id) = {
        let session_guard = session.read().await;
        (session_guard.character_id, session_guard.room_id)
    };

    let char_id = match char_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    debug!(
        "One-time take: char_id={}, room={}, real_id={}",
        char_id, room_id, real_id
    );

    // Check if room has this one-time item
    let one_time = match db::get_one_time_item(&server.db, room_id, real_id).await? {
        Some(item) => item,
        None => {
            warn!(
                "One-time item {} not found in room {}",
                real_id, room_id
            );
            return Ok(vec![]);
        }
    };

    // Check if player already took this item
    if db::has_taken_one_time(&server.db, char_id, room_id, real_id).await? {
        warn!(
            "Player {} already took one-time item {} in room {}",
            char_id, real_id, room_id
        );
        return Ok(vec![]);
    }

    // Load inventory
    let inventory = match db::get_inventory(&server.db, char_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };

    // Find free slot based on category
    let (free_slot, current_items) = match one_time.category {
        CAT_OUTFIT => {
            let items = inventory.outfits();
            (items.iter().position(|&x| x == 0), items)
        }
        CAT_ITEM => {
            let items = inventory.items();
            (items.iter().position(|&x| x == 0), items)
        }
        CAT_ACCESSORY => {
            let items = inventory.accessories();
            (items.iter().position(|&x| x == 0), items)
        }
        _ => return Ok(vec![]),
    };

    let free_slot = match free_slot {
        Some(idx) => idx,
        None => {
            warn!(
                "Player {} has no free slot for one-time item (category {})",
                char_id, one_time.category
            );
            return Ok(vec![]);
        }
    };

    // Mark item as taken
    let _ = db::mark_one_time_taken(&server.db, char_id, room_id, real_id).await;

    // Add item to inventory
    let mut new_items = current_items;
    new_items[free_slot] = one_time.item_id;

    match one_time.category {
        CAT_OUTFIT => {
            let _ = db::update_inventory_outfits(&server.db, char_id, &new_items).await;
        }
        CAT_ITEM => {
            let _ = db::update_inventory_items(&server.db, char_id, &new_items).await;
        }
        CAT_ACCESSORY => {
            let _ = db::update_inventory_accessories(&server.db, char_id, &new_items).await;
        }
        _ => {}
    }

    info!(
        "Player {} took one-time item {} (category {}, slot {})",
        char_id, one_time.item_id, one_time.category, free_slot + 1
    );

    // Send response (uses different message type than request)
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::OneTimeGet.id());
    writer.write_u8(one_time.category);
    writer.write_u8((free_slot + 1) as u8);
    writer.write_u16(one_time.item_id);

    Ok(vec![writer.into_bytes()])
}
