//! Collectible system handlers
//!
//! Handles world collectibles (materials like mushrooms etc.)
//! that spawn at fixed locations and respawn after being taken.
//!
//! Protocol:
//! - MSG_COLLECTIBLE_INFO (32): Server → Client when entering room
//! - MSG_COLLECTIBLE_TAKE_SELF (33): Client → Server when picking up
//! - MSG_COLLECTIBLE_TAKEN (34): Server → Other clients in room
//! - MSG_GET_ITEM (41): Server → Client to give item

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::game::{CollectibleSpawn, PlayerSession};
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

/// Default respawn time in seconds for collectibles
pub const DEFAULT_RESPAWN_SECS: u32 = 300; // 5 minutes

/// Get collectible spawn definitions for a room
///
/// This function returns the spawn points for collectibles in each room.
/// Add more rooms here as the game world is documented.
pub fn get_room_collectibles(room_id: u16) -> Vec<CollectibleSpawn> {
    match room_id {
        // Deep Woods area - mushrooms and materials
        // Room IDs are from the game's room index

        // Example spawn points (to be filled in with actual game data)
        // The spawn point ID (col_id) must be unique within each room (0-255)

        // Deep Woods 1 (example)
        1 => vec![
            CollectibleSpawn {
                col_id: 0,
                item_id: 20, // Red Mushroom
                x: 200,
                y: 400,
                respawn_secs: Some(DEFAULT_RESPAWN_SECS),
            },
            CollectibleSpawn {
                col_id: 1,
                item_id: 58, // Squishy Mushroom
                x: 450,
                y: 350,
                respawn_secs: Some(DEFAULT_RESPAWN_SECS),
            },
        ],

        // Deep Woods 2 (example)
        2 => vec![CollectibleSpawn {
            col_id: 0,
            item_id: 59, // Stinky Mushroom
            x: 300,
            y: 420,
            respawn_secs: Some(DEFAULT_RESPAWN_SECS),
        }],

        // Swamp area - Irrlicht (will-o-wisp)
        // These use item_id 61 which has special rendering
        10 => vec![CollectibleSpawn {
            col_id: 0,
            item_id: 61, // Irrlicht
            x: 150,
            y: 300,
            respawn_secs: Some(DEFAULT_RESPAWN_SECS * 2), // Rare, slower respawn
        }],

        // Volcano/Fire area - Blazing Bubbles, Firestones
        20 => vec![
            CollectibleSpawn {
                col_id: 0,
                item_id: 57, // Blazing Bubble
                x: 400,
                y: 280,
                respawn_secs: Some(DEFAULT_RESPAWN_SECS),
            },
            CollectibleSpawn {
                col_id: 1,
                item_id: 50, // Firestone
                x: 550,
                y: 320,
                respawn_secs: Some(DEFAULT_RESPAWN_SECS),
            },
        ],

        // Crystal/Gem area - Tailphire, Magmanis
        30 => vec![
            CollectibleSpawn {
                col_id: 0,
                item_id: 21, // Tailphire
                x: 250,
                y: 350,
                respawn_secs: Some(DEFAULT_RESPAWN_SECS),
            },
            CollectibleSpawn {
                col_id: 1,
                item_id: 22, // Magmanis
                x: 500,
                y: 380,
                respawn_secs: Some(DEFAULT_RESPAWN_SECS),
            },
        ],

        // Factory/Tech area - Screws
        40 => vec![
            CollectibleSpawn {
                col_id: 0,
                item_id: 46, // Screw
                x: 180,
                y: 400,
                respawn_secs: Some(DEFAULT_RESPAWN_SECS),
            },
            CollectibleSpawn {
                col_id: 1,
                item_id: 47, // Rusty Screw
                x: 420,
                y: 380,
                respawn_secs: Some(DEFAULT_RESPAWN_SECS),
            },
        ],

        // No collectibles in this room
        _ => vec![],
    }
}

/// Initialize collectibles for a room when a player enters
/// Called when a player enters a room that hasn't been initialized yet
pub async fn init_room_if_needed(server: &Arc<Server>, room_id: u16) {
    // Check if room already has collectibles initialized
    if let Some(room) = server.game_state.get_room(room_id) {
        let collectibles = room.collectibles.read().await;
        if !collectibles.is_empty() {
            // Already initialized
            return;
        }
    }

    // Get spawn definitions for this room
    let spawns = get_room_collectibles(room_id);
    if !spawns.is_empty() {
        server
            .game_state
            .init_room_collectibles(room_id, spawns)
            .await;
        debug!("Initialized collectibles for room {}", room_id);
    }
}

/// Write MSG_COLLECTIBLE_INFO message for a room
/// Called when a player enters a room to tell them about available collectibles
pub async fn write_collectible_info(server: &Arc<Server>, room_id: u16) -> Vec<u8> {
    // Initialize room collectibles if needed
    init_room_if_needed(server, room_id).await;

    // Get available collectibles
    let available = server.game_state.get_available_collectibles(room_id).await;

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

    writer.into_bytes()
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

    // Try to take the collectible
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
    if let Err(e) = crate::db::update_item_slot(&server.db, character_id, slot, spawn.item_id as i16).await {
        warn!(
            "Failed to add collectible item {} to player {}: {}",
            spawn.item_id, player_id, e
        );
        return Ok(vec![]);
    }

    // Send MSG_GET_ITEM to the player to confirm they got the item
    let mut item_writer = MessageWriter::new();
    item_writer.write_u16(MessageType::GetItem.id());
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
