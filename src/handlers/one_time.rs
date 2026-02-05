//! One-time item handlers
//!
//! Handles one-time collectible items that can only be taken once per player:
//! - MSG_ONE_TIME_GET (37) - Take a one-time item
//! - MSG_ONE_TIME_INFO (35) - Send available one-time items in a room
//! - MSG_ONE_TIME_DISAPPEAR (36) - Remove one-time items that are no longer available

use std::sync::Arc;

use anyhow::Result;
use chrono::Timelike;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::Server;
use crate::db;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};

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
            warn!("One-time item {} not found in room {}", real_id, room_id);
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
        char_id,
        one_time.item_id,
        one_time.category,
        free_slot + 1
    );

    // Send response (uses different message type than request)
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::OneTimeGet.id());
    writer.write_u8(one_time.category);
    writer.write_u8((free_slot + 1) as u8);
    writer.write_u16(one_time.item_id);

    Ok(vec![writer.into_bytes()])
}

/// Send MSG_ONE_TIME_INFO (35) messages for all available one-time items in a room
/// that the character has not yet taken
pub async fn write_room_onetimes(
    server: &Arc<Server>,
    room_id: u16,
    character_id: i64,
) -> Vec<Vec<u8>> {
    let mut messages = Vec::new();

    // Get current hour (0-23)
    let now = chrono::Utc::now();
    let current_hour = now.hour() as u8;

    // Get available items from config
    let available_items = server
        .game_config
        .onetimes
        .get_available_items(room_id, current_hour);

    if available_items.is_empty() {
        return messages;
    }

    debug!(
        "Checking {} one-time items in room {} for character {}",
        available_items.len(),
        room_id,
        character_id
    );

    // Filter out items the player has already taken
    for item in available_items {
        // Check if player already took this item
        match db::has_taken_one_time(&server.db, character_id, room_id, item.real_id).await {
            Ok(true) => {
                // Player already took this item, skip it
                continue;
            }
            Ok(false) => {
                // Player hasn't taken it, send the info
                let mut writer = MessageWriter::new();
                writer
                    .write_u16(MessageType::OneTimeInfo.id())
                    .write_u16(item.x)
                    .write_u16(item.y)
                    .write_u8(item.real_id)
                    .write_u16(item.item_id)
                    .write_u8(item.category);

                messages.push(writer.into_bytes());

                debug!(
                    "Sent one-time item {} (category {}, id {}) at ({}, {}) in room {}",
                    item.real_id, item.category, item.item_id, item.x, item.y, room_id
                );
            }
            Err(e) => {
                warn!(
                    "Failed to check if character {} took one-time item {}: {}",
                    character_id, item.real_id, e
                );
            }
        }
    }

    messages
}

/// Check all players in a room and send MSG_ONE_TIME_DISAPPEAR (36) for items
/// that are no longer available based on the current hour
pub async fn check_hourly_onetimes(server: &Arc<Server>) {
    let now = chrono::Utc::now();
    let current_hour = now.hour() as u8;

    debug!(
        "Checking one-time item availability at hour {}",
        current_hour
    );

    // For each room with one-time items
    for (room_id, room_config) in &server.game_config.onetimes.rooms {
        // Get all players in this room
        let players = server.game_state.get_room_players(*room_id).await;
        if players.is_empty() {
            continue;
        }

        // Check each one-time item in the room
        for item in &room_config.items {
            // Skip items that are always available
            if item.start_hour == 0 && item.end_hour == 0 {
                continue;
            }

            // If item is NOT available at current hour, send disappear message
            if !item.is_available_at(current_hour) {
                debug!(
                    "One-time item {} in room {} is no longer available at hour {}",
                    item.real_id, room_id, current_hour
                );

                // Send disappear message to all players in the room
                for player_id in &players {
                    if let Some(session_id) = server.game_state.players_by_id.get(player_id)
                        && let Some(handle) = server.sessions.get(&session_id)
                    {
                        let mut writer = MessageWriter::new();
                        writer
                            .write_u16(MessageType::OneTimeDisappear.id())
                            .write_u8(item.real_id);

                        handle.queue_message(writer.into_bytes()).await;
                    }
                }
            }
        }
    }
}
