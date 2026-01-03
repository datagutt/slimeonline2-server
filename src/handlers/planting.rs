//! Planting system handlers
//!
//! Handles plant-related messages:
//! - MSG_PLANT_SET (65) - Plant a seed
//! - MSG_PLANT_ADD_PINWHEEL (68) - Add pinwheel to plant
//! - MSG_PLANT_ADD_FAIRY (69) - Add fairy to plant
//! - MSG_PLANT_TAKE_FRUIT (71) - Take fruit from plant

use std::sync::Arc;

use anyhow::Result;
use chrono::{Duration, Utc};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::db;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

/// Valid seed item IDs
const SEED_BASIC: u16 = 9;
const SEED_BLUE: u16 = 24;

/// Valid pinwheel item IDs (11, 12, 13)
const PINWHEEL_IDS: [u16; 3] = [11, 12, 13];

/// Fairy item ID
const FAIRY_ID: u16 = 10;

/// Maximum fairies per plant
const MAX_FAIRIES: u8 = 5;

/// Handle MSG_PLANT_SET (65) - Plant a seed
pub async fn handle_plant_set(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let seed_slot = reader.read_u8()?;
    let plant_spot = reader.read_u8()?;

    debug!(
        "Plant set request: seed_slot={}, plant_spot={}",
        seed_slot, plant_spot
    );

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
    if seed_slot < 1 || seed_slot > 9 {
        warn!("Invalid seed slot: {}", seed_slot);
        return Ok(vec![]);
    }

    // Load inventory from database
    let inventory = match db::get_inventory(&server.db, char_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };

    let items = inventory.items();
    let slot_idx = (seed_slot - 1) as usize;
    let seed_id = items[slot_idx];

    // Check if slot has an item
    if seed_id == 0 {
        warn!("Seed slot {} is empty", seed_slot);
        return Ok(vec![]);
    }

    // Check if item is a valid seed
    if seed_id != SEED_BASIC && seed_id != SEED_BLUE {
        warn!("Item {} in slot {} is not a valid seed", seed_id, seed_slot);
        return Ok(vec![]);
    }

    // Check if spot is free (no existing plant)
    if let Ok(Some(_)) = db::get_plant_state(&server.db, room_id, plant_spot).await {
        // Spot is occupied, return the seed to player
        warn!(
            "Plant spot {} in room {} is already occupied",
            plant_spot, room_id
        );
        let mut writer = MessageWriter::new();
        writer.write_u16(MessageType::GetItem.id());
        writer.write_u8(seed_slot);
        writer.write_u16(seed_id);
        return Ok(vec![writer.into_bytes()]);
    }

    // Get growth time from config
    let growth_time = get_growth_time(seed_id, 0, 0);
    let next_stage_at = Utc::now() + Duration::minutes(growth_time as i64);

    // Plant the seed
    if let Err(e) = db::plant_seed(&server.db, room_id, plant_spot, char_id, seed_id, next_stage_at)
        .await
    {
        warn!("Failed to plant seed: {}", e);
        return Ok(vec![]);
    }

    // Remove seed from player's inventory
    let mut new_items = items;
    new_items[slot_idx] = 0;
    if let Err(e) = db::update_inventory_items(&server.db, char_id, &new_items).await {
        warn!("Failed to update inventory: {}", e);
    }

    // Increment trees planted count
    if let Err(e) = db::increment_trees_planted(&server.db, char_id).await {
        warn!("Failed to increment trees planted: {}", e);
    }

    let mut responses = Vec::new();

    // Send MSG_TREE_PLANTED_INC to player
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::TreePlantedInc.id());
    responses.push(writer.into_bytes());

    // Broadcast MSG_PLANT_SPOT_USED to room
    let mut broadcast = MessageWriter::new();
    broadcast.write_u16(MessageType::PlantSpotUsed.id());
    broadcast.write_u8(plant_spot);
    broadcast.write_u16(player_id);
    broadcast.write_u16(seed_id);
    broadcast.write_u8(0); // step
    broadcast.write_u8(0); // wheel
    broadcast.write_u8(0); // fairies
    let msg = broadcast.into_bytes();

    // Broadcast to all players in room
    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(other_session_id.value()) {
                other_session.write().await.queue_message(msg.clone());
            }
        }
    }

    info!(
        "Player {} planted seed {} at spot {} in room {}",
        char_id, seed_id, plant_spot, room_id
    );

    Ok(responses)
}

/// Handle MSG_PLANT_ADD_PINWHEEL (68) - Add pinwheel to plant
pub async fn handle_plant_add_pinwheel(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let item_slot = reader.read_u8()?;
    let plant_spot = reader.read_u8()?;

    let (char_id, room_id) = {
        let session_guard = session.read().await;
        (session_guard.character_id, session_guard.room_id)
    };

    let char_id = match char_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Validate slot
    if item_slot < 1 || item_slot > 9 {
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

    // Check if slot has a pinwheel
    if !PINWHEEL_IDS.contains(&item_id) {
        warn!("Item {} is not a pinwheel", item_id);
        return Ok(vec![]);
    }

    // Check plant ownership
    let plant = match db::get_plant_state(&server.db, room_id, plant_spot).await? {
        Some(p) => p,
        None => {
            warn!("No plant at spot {} in room {}", plant_spot, room_id);
            return Ok(vec![]);
        }
    };

    if plant.owner_id != Some(char_id) {
        warn!(
            "Player {} doesn't own plant at spot {}",
            char_id, plant_spot
        );
        return Ok(vec![]);
    }

    // Remove pinwheel from inventory
    let mut new_items = items;
    new_items[slot_idx] = 0;
    let _ = db::update_inventory_items(&server.db, char_id, &new_items).await;

    // Add pinwheel to plant and update growth time
    let _ = db::add_pinwheel_to_plant(&server.db, room_id, plant_spot, item_id).await;

    // Recalculate growth time with pinwheel bonus
    let seed_id = plant.seed_id.unwrap_or(SEED_BASIC as i64) as u16;
    let new_growth_time = get_growth_time(seed_id, plant.fairy_count as u8, item_id as u8);
    let next_stage_at = Utc::now() + Duration::minutes(new_growth_time as i64);
    let _ = db::update_plant_next_stage(&server.db, room_id, plant_spot, next_stage_at).await;

    // Broadcast to room
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::PlantAddPinwheel.id());
    writer.write_u8(plant_spot);
    writer.write_u8(item_id as u8);
    let msg = writer.into_bytes();

    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(other_session_id.value()) {
                other_session.write().await.queue_message(msg.clone());
            }
        }
    }

    info!(
        "Player {} added pinwheel {} to plant at spot {} in room {}",
        char_id, item_id, plant_spot, room_id
    );

    Ok(vec![])
}

/// Handle MSG_PLANT_ADD_FAIRY (69) - Add fairy to plant
pub async fn handle_plant_add_fairy(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let item_slot = reader.read_u8()?;
    let plant_spot = reader.read_u8()?;

    let (char_id, room_id) = {
        let session_guard = session.read().await;
        (session_guard.character_id, session_guard.room_id)
    };

    let char_id = match char_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Validate slot
    if item_slot < 1 || item_slot > 9 {
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

    // Check if slot has a fairy
    if item_id != FAIRY_ID {
        warn!("Item {} is not a fairy", item_id);
        return Ok(vec![]);
    }

    // Check plant ownership and fairy count
    let plant = match db::get_plant_state(&server.db, room_id, plant_spot).await? {
        Some(p) => p,
        None => {
            warn!("No plant at spot {} in room {}", plant_spot, room_id);
            return Ok(vec![]);
        }
    };

    if plant.owner_id != Some(char_id) {
        warn!(
            "Player {} doesn't own plant at spot {}",
            char_id, plant_spot
        );
        return Ok(vec![]);
    }

    if plant.fairy_count >= MAX_FAIRIES as i64 {
        warn!("Plant already has max fairies");
        return Ok(vec![]);
    }

    // Remove fairy from inventory
    let mut new_items = items;
    new_items[slot_idx] = 0;
    let _ = db::update_inventory_items(&server.db, char_id, &new_items).await;

    // Add fairy to plant
    let _ = db::add_fairy_to_plant(&server.db, room_id, plant_spot).await;

    // Broadcast to room
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::PlantAddFairy.id());
    writer.write_u8(plant_spot);
    let msg = writer.into_bytes();

    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(other_session_id.value()) {
                other_session.write().await.queue_message(msg.clone());
            }
        }
    }

    info!(
        "Player {} added fairy to plant at spot {} in room {}",
        char_id, plant_spot, room_id
    );

    Ok(vec![])
}

/// Handle MSG_PLANT_TAKE_FRUIT (71) - Take fruit from plant
pub async fn handle_plant_take_fruit(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let plant_spot = reader.read_u8()?;
    let fruit_slot = reader.read_u8()?; // 1, 2, or 3

    let (char_id, room_id) = {
        let session_guard = session.read().await;
        (session_guard.character_id, session_guard.room_id)
    };

    let char_id = match char_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Validate fruit slot (1-3)
    if fruit_slot < 1 || fruit_slot > 3 {
        warn!("Invalid fruit slot: {}", fruit_slot);
        return Ok(vec![]);
    }

    // Load inventory from database to find free slot
    let inventory = match db::get_inventory(&server.db, char_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };

    let items = inventory.items();
    let free_slot = items.iter().position(|&x| x == 0);
    let free_slot = match free_slot {
        Some(idx) => idx,
        None => {
            warn!("Player has no free item slot");
            return Ok(vec![]);
        }
    };

    // Get plant and verify ownership
    let plant = match db::get_plant_state(&server.db, room_id, plant_spot).await? {
        Some(p) => p,
        None => {
            warn!("No plant at spot {} in room {}", plant_spot, room_id);
            return Ok(vec![]);
        }
    };

    if plant.owner_id != Some(char_id) {
        warn!(
            "Player {} doesn't own plant at spot {}",
            char_id, plant_spot
        );
        return Ok(vec![]);
    }

    // Check plant has fruit
    if plant.has_fruit == 0 {
        warn!("Plant at spot {} has no fruit", plant_spot);
        return Ok(vec![]);
    }

    // Get fruit item based on seed type
    let fruit_item = get_fruit_for_plant(&plant, fruit_slot);

    if fruit_item == 0 {
        warn!("No fruit in slot {}", fruit_slot);
        return Ok(vec![]);
    }

    // Mark fruit as taken
    let _ = db::take_plant_fruit(&server.db, room_id, plant_spot, fruit_slot).await;

    // Add fruit to player inventory
    let mut new_items = items;
    new_items[free_slot] = fruit_item;
    let _ = db::update_inventory_items(&server.db, char_id, &new_items).await;

    let mut responses = Vec::new();

    // Send MSG_GET_ITEM to player
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::GetItem.id());
    writer.write_u8((free_slot + 1) as u8);
    writer.write_u16(fruit_item);
    responses.push(writer.into_bytes());

    // Broadcast MSG_PLANT_TAKE_FRUIT to room
    let mut broadcast = MessageWriter::new();
    broadcast.write_u16(MessageType::PlantTakeFruit.id());
    broadcast.write_u8(plant_spot);
    broadcast.write_u8(fruit_slot);
    let msg = broadcast.into_bytes();

    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(other_session_id.value()) {
                other_session.write().await.queue_message(msg.clone());
            }
        }
    }

    // Check if all fruits are taken - advance to step 5
    let updated_plant = db::get_plant_state(&server.db, room_id, plant_spot).await?;
    if let Some(p) = updated_plant {
        // If has_fruit is now 0 after all fruits taken, advance stage
        if p.has_fruit == 0 && p.stage < 5 {
            let growth_time = get_growth_time(
                p.seed_id.unwrap_or(SEED_BASIC as i64) as u16,
                p.fairy_count as u8,
                p.pinwheel_id.unwrap_or(0) as u8,
            );
            let next_stage_at = Utc::now() + Duration::minutes(growth_time as i64);
            let _ = db::advance_plant_stage(
                &server.db,
                room_id,
                plant_spot,
                5,
                Some(next_stage_at),
                false,
            )
            .await;

            // Broadcast stage change
            let mut stage_msg = MessageWriter::new();
            stage_msg.write_u16(MessageType::PlantGrow.id());
            stage_msg.write_u8(plant_spot);
            stage_msg.write_u8(5);
            let msg = stage_msg.into_bytes();

            for other_player_id in server.game_state.get_room_players(room_id).await {
                if let Some(other_session_id) =
                    server.game_state.players_by_id.get(&other_player_id)
                {
                    if let Some(other_session) = server.sessions.get(other_session_id.value()) {
                        other_session.write().await.queue_message(msg.clone());
                    }
                }
            }
        }
    }

    info!(
        "Player {} took fruit {} from plant at spot {} in room {}",
        char_id, fruit_slot, plant_spot, room_id
    );

    Ok(responses)
}

/// Send plant state when player enters a room
pub async fn send_room_plants(server: &Arc<Server>, room_id: u16) -> Vec<Vec<u8>> {
    let mut responses = Vec::new();

    let plants = match db::get_plant_states(&server.db, room_id).await {
        Ok(p) => p,
        Err(_) => return responses,
    };

    for plant in plants {
        if plant.owner_id.is_none() {
            // Empty spot - send MSG_PLANT_SPOT_FREE
            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::PlantSpotFree.id());
            writer.write_u8(plant.spot_id as u8);
            responses.push(writer.into_bytes());
        } else {
            // Occupied spot - send MSG_PLANT_SPOT_USED
            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::PlantSpotUsed.id());
            writer.write_u8(plant.spot_id as u8);
            writer.write_u16(plant.owner_id.unwrap_or(0) as u16);
            writer.write_u16(plant.seed_id.unwrap_or(0) as u16);
            writer.write_u8(plant.stage as u8);
            writer.write_u8(plant.pinwheel_id.unwrap_or(0) as u8);
            writer.write_u8(plant.fairy_count as u8);
            responses.push(writer.into_bytes());

            // If plant has fruit, send MSG_PLANT_HAS_FRUIT
            if plant.has_fruit != 0 {
                let (fruit1, fruit2, fruit3) = get_all_fruits_for_plant(&plant);
                let mut fruit_writer = MessageWriter::new();
                fruit_writer.write_u16(MessageType::PlantHasFruit.id());
                fruit_writer.write_u8(plant.spot_id as u8);
                fruit_writer.write_u16(fruit1);
                fruit_writer.write_u16(fruit2);
                fruit_writer.write_u16(fruit3);
                responses.push(fruit_writer.into_bytes());
            }
        }
    }

    responses
}

/// Get growth time in minutes based on seed, fairies, and pinwheel
fn get_growth_time(seed_id: u16, _fairies: u8, pinwheel: u8) -> u32 {
    // Get base time from config
    let base_time = match seed_id {
        SEED_BASIC => 240, // 4 hours default
        SEED_BLUE => 360,  // 6 hours default
        _ => 240,
    };

    // Apply pinwheel bonus
    let pinwheel_multiplier = match pinwheel {
        11 => 0.80, // 20% faster
        12 => 0.65, // 35% faster
        13 => 0.50, // 50% faster
        _ => 1.0,
    };

    (base_time as f64 * pinwheel_multiplier) as u32
}

/// Get a fruit item for a plant at a specific slot
fn get_fruit_for_plant(plant: &db::PlantState, slot: u8) -> u16 {
    // Get fruit based on seed type
    let seed_id = plant.seed_id.unwrap_or(SEED_BASIC as i64) as u16;
    match seed_id {
        SEED_BASIC => SEED_BASIC, // Basic seed produces basic seeds
        SEED_BLUE => {
            // Blue seed produces mostly Juicy Bango (25), sometimes Blue Seed (24)
            if slot == 2 {
                24
            } else {
                25
            }
        }
        _ => SEED_BASIC,
    }
}

/// Get all three fruits for a plant
fn get_all_fruits_for_plant(plant: &db::PlantState) -> (u16, u16, u16) {
    (
        get_fruit_for_plant(plant, 1),
        get_fruit_for_plant(plant, 2),
        get_fruit_for_plant(plant, 3),
    )
}
