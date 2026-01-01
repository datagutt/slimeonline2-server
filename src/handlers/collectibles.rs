//! Collectible system handlers
//!
//! Handles world collectibles (materials like mushrooms etc.)
//! that spawn at fixed locations and respawn after being taken.
//!
//! Protocol:
//! - MSG_COLLECTIBLE_INFO (32): Server → Client when entering room
//! - MSG_COLLECTIBLE_TAKE_SELF (34): Client → Server when picking up
//! - MSG_COLLECTIBLE_TAKEN (33): Server → Other clients in room
//! - MSG_GET_ITEM (41): Server → Client to give item

use std::sync::Arc;

use anyhow::Result;
use chrono::{Duration, Utc};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::config::GameConfig;
use crate::game::{CollectibleSpawn, PlayerSession};
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

/// Get collectible spawn definitions for a room from config
///
/// This function returns the spawn points for collectibles in each room.
/// Spawn points are defined in config/collectibles.toml.
///
/// Respawn times include randomized variance for each spawn.
pub fn get_room_collectibles(config: &GameConfig, room_id: u16) -> Vec<CollectibleSpawn> {
    use rand::Rng;

    match config.collectibles.get_room(room_id) {
        Some(room_config) => {
            let mut rng = rand::thread_rng();

            room_config
                .spawns
                .iter()
                .map(|spawn| {
                    // Convert minutes to seconds
                    let base_respawn_secs = spawn.respawn * 60;
                    let variance_secs = spawn.variance * 60;

                    // Add random variance: base + random(0, variance)
                    let random_variance = if variance_secs > 0 {
                        rng.gen_range(0..=variance_secs)
                    } else {
                        0
                    };
                    let total_respawn = base_respawn_secs + random_variance;

                    CollectibleSpawn {
                        col_id: spawn.id,
                        item_id: spawn.item,
                        x: spawn.x,
                        y: spawn.y,
                        respawn_secs: Some(total_respawn),
                    }
                })
                .collect()
        }
        None => vec![],
    }
}

/// Initialize collectibles for a room when a player enters
/// Called when a player enters a room that hasn't been initialized yet
/// Loads state from database to restore availability across restarts
pub async fn init_room_if_needed(server: &Arc<Server>, room_id: u16) {
    // Check if room already has collectibles initialized in memory
    if let Some(room) = server.game_state.get_room(room_id) {
        let collectibles = room.collectibles.read().await;
        if !collectibles.is_empty() {
            // Already initialized
            return;
        }
    }

    // Get spawn definitions for this room from config
    let spawns = get_room_collectibles(&server.game_config, room_id);
    if spawns.is_empty() {
        return;
    }

    // Load persisted state from database
    let db_states = match crate::db::get_collectible_states(&server.db, room_id).await {
        Ok(states) => states,
        Err(e) => {
            warn!(
                "Failed to load collectible state for room {}: {}",
                room_id, e
            );
            vec![]
        }
    };

    // Apply database state to spawns and initialize room
    // Build a map of spawn_id -> db state for quick lookup
    let state_map: std::collections::HashMap<u8, _> = db_states
        .into_iter()
        .map(|s| (s.spawn_id as u8, s))
        .collect();

    // Initialize room with spawns, marking unavailable ones based on DB
    server
        .game_state
        .init_room_collectibles_with_state(room_id, spawns, state_map)
        .await;

    debug!(
        "Initialized collectibles for room {} from config + database",
        room_id
    );
}

/// Write MSG_COLLECTIBLE_INFO message for a room
/// Called when a player enters a room to tell them about available collectibles
/// Returns None if there are no collectibles in the room
pub async fn write_collectible_info(server: &Arc<Server>, room_id: u16) -> Option<Vec<u8>> {
    // Initialize room collectibles if needed
    init_room_if_needed(server, room_id).await;

    // Get available collectibles
    let available = server.game_state.get_available_collectibles(room_id).await;

    // Don't send the message if there are no collectibles
    if available.is_empty() {
        return None;
    }

    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::CollectibleInfo.id());

    // Count (u8) - max 255 collectibles per room
    let count = std::cmp::min(available.len(), 255) as u8;
    writer.write_u8(count);

    // For each available collectible
    for col in available.iter().take(count as usize) {
        writer.write_u8(col.spawn.col_id); // col_id (u8)
        writer.write_u16(col.spawn.item_id); // item_id (u16)
        writer.write_u16(col.spawn.x); // x (u16)
        writer.write_u16(col.spawn.y); // y (u16)
    }

    Some(writer.into_bytes())
}

/// Handle MSG_COLLECTIBLE_TAKE_SELF (33)
/// Player is trying to pick up a collectible
///
/// Format: col_id (u8)
pub async fn handle_collectible_take(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);

    let col_id = reader.read_u8()?;

    let (player_id, room_id, character_id) = {
        let session_guard = session.read().await;
        (
            session_guard.player_id,
            session_guard.room_id,
            session_guard.character_id,
        )
    };

    let player_id = match player_id {
        Some(id) => id,
        None => {
            warn!("Unauthenticated player tried to take collectible");
            return Ok(vec![]);
        }
    };

    let character_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Try to take the collectible (updates in-memory state)
    let spawn = match server.game_state.take_collectible(room_id, col_id).await {
        Some(spawn) => spawn,
        None => {
            // Collectible doesn't exist or was already taken
            debug!(
                "Player {} tried to take unavailable collectible {} in room {}",
                player_id, col_id, room_id
            );
            return Ok(vec![]);
        }
    };

    debug!(
        "Player {} took collectible {} (item {}) in room {}",
        player_id, col_id, spawn.item_id, room_id
    );

    // Persist the taken state to database with respawn time
    let respawn_secs = spawn.respawn_secs.unwrap_or(3600); // Default 1 hour if not specified
    let respawn_at = Utc::now() + Duration::seconds(respawn_secs as i64);

    if let Err(e) = crate::db::take_collectible(&server.db, room_id, col_id, respawn_at).await {
        warn!("Failed to persist collectible state: {}", e);
        // Continue anyway - the in-memory state is updated
    }

    // Find a free slot in player's inventory and give item
    let inventory_result = crate::db::get_inventory(&server.db, character_id).await;

    let slot = match inventory_result {
        Ok(Some(inv)) => {
            // Find first empty slot in items (1-9)
            let items = inv.items();
            let mut free_slot = None;
            for (i, item_id) in items.iter().enumerate() {
                if *item_id == 0 {
                    free_slot = Some((i + 1) as u8); // Slots are 1-indexed
                    break;
                }
            }
            free_slot
        }
        Ok(None) => Some(1), // No inventory yet, slot 1 is free
        Err(e) => {
            warn!("Failed to get inventory for player {}: {}", player_id, e);
            return Ok(vec![]);
        }
    };

    let slot = match slot {
        Some(s) => s,
        None => {
            // No free slot - this shouldn't happen as client checks first
            // But the collectible is already taken, so we need to put it back
            // Actually, don't put it back - the client already destroyed it visually
            // Just don't give the item
            warn!("Player {} has no free slots for collectible", player_id);
            return Ok(vec![]);
        }
    };

    // Add item to inventory at the found slot
    if let Err(e) =
        crate::db::update_item_slot(&server.db, character_id, slot, spawn.item_id as i16).await
    {
        warn!(
            "Failed to add collectible item {} to player {}: {}",
            spawn.item_id, player_id, e
        );
        return Ok(vec![]);
    }

    // Send MSG_COLLECTIBLE_TAKE_SELF to the player to confirm they got the item
    // Format: slot (u8) + item_id (u16)
    let mut item_writer = MessageWriter::new();
    item_writer.write_u16(MessageType::CollectibleTakeSelf.id());
    item_writer.write_u8(slot); // slot (u8)
    item_writer.write_u16(spawn.item_id); // item_id (u16)

    let responses = vec![item_writer.into_bytes()];

    // Broadcast MSG_COLLECTIBLE_TAKEN to other players in the room
    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut taken_writer = MessageWriter::new();
                taken_writer.write_u16(MessageType::CollectibleTaken.id());
                taken_writer.write_u8(col_id);
                other_session
                    .write()
                    .await
                    .queue_message(taken_writer.into_bytes());
            }
        }
    }

    Ok(responses)
}
