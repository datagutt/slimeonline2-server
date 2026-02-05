//! Building system handlers
//!
//! Handles building-related messages:
//! - MSG_BUILD_OBJECT (105) - Place a building on a spot
//!
//! Building spots are predefined locations in rooms where players can place
//! buildable items (like Weak Cannon Kit). Buildings expire after a set duration.

use std::sync::Arc;

use anyhow::Result;
use chrono::{Duration, Utc};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::Server;
use crate::db;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};

/// Handle MSG_BUILD_OBJECT (105) - Player wants to place a building
pub async fn handle_build_object(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let item_slot = reader.read_u8()?;
    let build_spot = reader.read_u8()?;

    let (char_id, player_id, room_id) = {
        let session_guard = session.read().await;
        (
            session_guard.character_id,
            session_guard.player_id,
            session_guard.room_id,
        )
    };

    let char_id = match char_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    let player_id = match player_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Validate slot range (1-9)
    if !(1..=9).contains(&item_slot) {
        warn!("Invalid build item slot: {}", item_slot);
        return Ok(vec![]);
    }

    // Check if this room has the specified building spot
    if !server.game_config.buildings.has_spot(room_id, build_spot) {
        warn!(
            "Room {} does not have building spot {}",
            room_id, build_spot
        );
        return Ok(vec![]);
    }

    // Load inventory from database
    let inventory = match db::get_inventory(&server.db, char_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };

    let items = inventory.items();
    let slot_idx = (item_slot - 1) as usize;
    let item_id = items[slot_idx];

    // Check if slot has an item
    if item_id == 0 {
        warn!("Build item slot {} is empty", item_slot);
        return Ok(vec![]);
    }

    // Check if item is a buildable object
    let build_config = match server.game_config.buildings.get_object(item_id) {
        Some(config) => config,
        None => {
            warn!("Item {} is not a buildable object", item_id);
            return Ok(vec![]);
        }
    };

    // Check if spot is free
    if let Ok(Some(existing)) = db::get_building_state(&server.db, room_id, build_spot).await {
        if !existing.is_free() {
            // Spot is occupied, return the item to player
            warn!(
                "Building spot {} in room {} is already occupied",
                build_spot, room_id
            );
            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::GetItem.id());
            writer.write_u8(item_slot);
            writer.write_u16(item_id);
            return Ok(vec![writer.into_bytes()]);
        }
    }

    // Calculate expiration time
    let expires_at = Utc::now() + Duration::hours(build_config.duration_hours as i64);

    // Place the building in database
    if let Err(e) = db::place_building(
        &server.db, room_id, build_spot, char_id, item_id, expires_at,
    )
    .await
    {
        warn!("Failed to place building: {}", e);
        return Ok(vec![]);
    }

    // Remove item from player's inventory
    let mut new_items = items;
    new_items[slot_idx] = 0;
    if let Err(e) = db::update_inventory_items(&server.db, char_id, &new_items).await {
        warn!("Failed to update inventory: {}", e);
    }

    // Increment objects_built counter
    if let Err(e) = db::increment_objects_built(&server.db, char_id).await {
        warn!("Failed to increment objects_built: {}", e);
    }

    let mut responses = Vec::new();

    // Send MSG_OBJECTS_BUILT_INC to the player
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::ObjectsBuiltInc.id());
    responses.push(writer.into_bytes());

    // Broadcast MSG_BUILD_SPOT_USED to all players in room
    let mut broadcast = MessageWriter::new();
    broadcast.write_u16(MessageType::BuildSpotUsed.id());
    broadcast.write_u8(build_spot);
    broadcast.write_u16(item_id); // object_id
    broadcast.write_u16(player_id); // owner
    let msg = broadcast.into_bytes();

    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id)
            && let Some(other_session) = server.sessions.get(other_session_id.value())
        {
            other_session.queue_message(msg.clone()).await;
        }
    }

    info!(
        "Player {} built {} at spot {} in room {} (expires at {})",
        char_id,
        build_config.name,
        build_spot,
        room_id,
        expires_at.format("%Y-%m-%d %H:%M:%S")
    );

    Ok(responses)
}

/// Send building state when player enters a room
pub async fn send_room_buildings(server: &Arc<Server>, room_id: u16) -> Vec<Vec<u8>> {
    let mut responses = Vec::new();

    // Get all spot IDs configured for this room
    let spot_ids = server.game_config.buildings.get_spot_ids(room_id);
    if spot_ids.is_empty() {
        return responses;
    }

    // Get building states from database
    let buildings = match db::get_building_states(&server.db, room_id).await {
        Ok(b) => b,
        Err(_) => return responses,
    };

    // Create a map of spot_id -> building state
    let building_map: std::collections::HashMap<u8, _> = buildings
        .into_iter()
        .map(|b| (b.spot_id as u8, b))
        .collect();

    // Send state for each configured spot
    for spot_id in spot_ids {
        if let Some(building) = building_map.get(&spot_id) {
            if building.is_free() {
                // Empty spot - send MSG_BUILD_SPOT_FREE
                let mut writer = MessageWriter::new();
                writer.write_u16(MessageType::BuildSpotFree.id());
                writer.write_u8(spot_id);
                responses.push(writer.into_bytes());
            } else {
                // Occupied spot - send MSG_BUILD_SPOT_USED
                let mut writer = MessageWriter::new();
                writer.write_u16(MessageType::BuildSpotUsed.id());
                writer.write_u8(spot_id);
                writer.write_u16(building.object_id.unwrap_or(0) as u16);
                writer.write_u16(building.owner_id.unwrap_or(0) as u16);
                responses.push(writer.into_bytes());
            }
        } else {
            // No database record - spot is free
            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::BuildSpotFree.id());
            writer.write_u8(spot_id);
            responses.push(writer.into_bytes());
        }
    }

    responses
}

/// Broadcast that a building spot has become free (building expired or destroyed)
pub async fn broadcast_building_freed(server: &Arc<Server>, room_id: u16, spot_id: u8) {
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BuildSpotBecomeFree.id());
    writer.write_u8(spot_id);
    let msg = writer.into_bytes();

    let room_players = server.game_state.get_room_players(room_id).await;
    for player_id in room_players {
        if let Some(session_id) = server.game_state.players_by_id.get(&player_id)
            && let Some(session) = server.sessions.get(session_id.value())
        {
            session.queue_message(msg.clone()).await;
        }
    }
}
