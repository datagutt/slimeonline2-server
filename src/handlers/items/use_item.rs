//! MSG_USE_ITEM (31) handler
//!
//! Client sends format varies by item type (see item_use_slot.gml):
//! - Most items: slot (1 byte) + x (2 bytes) + y (2 bytes)
//! - Slimebags/Chicken Mine/Bright Drink/Sodas: slot (1 byte) only
//! - Bubbles: slot (1 byte) + x (2 bytes) + y (2 bytes) + direction (1 byte)

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::anticheat::validate_position_bounds;
use crate::constants::ITEM_SLOTS;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::rate_limit::ActionType;
use crate::Server;

use super::database::{get_item_info, ItemType};

/// Handle MSG_USE_ITEM (31)
pub async fn handle_use_item(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let mut reader = MessageReader::new(payload);
    let slot = reader.read_u8()?;

    // Validate slot
    if slot < 1 || slot > ITEM_SLOTS as u8 {
        warn!("Invalid item slot: {}", slot);
        return Ok(vec![]);
    }

    let (character_id, player_id, room_id, session_id, session_x, session_y) = {
        let session_guard = session.read().await;
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }
        (
            session_guard.character_id,
            session_guard.player_id,
            session_guard.room_id,
            session_guard.session_id,
            session_guard.x,
            session_guard.y,
        )
    };

    // Rate limit item usage
    if !server.rate_limiter.check_player(session_id.as_u128() as u64, ActionType::UseItem)
        .await
        .is_allowed()
    {
        debug!("Item use rate limited for player {:?}", player_id);
        return Ok(vec![]);
    }

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
        debug!("Slot {} is empty", slot);
        return Ok(vec![]);
    }

    let item_info = match get_item_info(item_id) {
        Some(info) => info,
        None => {
            warn!("Unknown item ID: {}", item_id);
            return Ok(vec![]);
        }
    };

    info!(
        "Player {} using item {} ({}) from slot {}",
        player_id, item_id, item_info.name, slot
    );

    let mut responses = Vec::new();

    // Read additional data based on item type
    let (use_x, use_y) = if payload.len() >= 5 {
        (reader.read_u16().ok(), reader.read_u16().ok())
    } else {
        (None, None)
    };

    // Validate and use provided coordinates, fall back to session position
    let (x, y) = match (use_x, use_y) {
        (Some(ux), Some(uy)) => {
            if validate_position_bounds(ux, uy) {
                (ux, uy)
            } else {
                warn!("Invalid item use position: ({}, {}), using session position", ux, uy);
                (session_x, session_y)
            }
        }
        _ => (session_x, session_y),
    };

    // Handle item effects based on type
    match item_info.item_type {
        ItemType::WarpWing => {
            handle_warp_wing(&mut responses, server, &session, character_id).await?;
        }

        ItemType::Smokebomb | ItemType::Applebomb => {
            broadcast_visual_effect(server, room_id, item_id, x, y).await;
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::Bubbles => {
            let direction = reader.read_u8().unwrap_or(0);
            broadcast_bubbles(server, room_id, item_id, x, y, direction).await;
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::Slimebag50 | ItemType::Slimebag200 | ItemType::Slimebag500 => {
            handle_slimebag(&item_info.item_type, server, &session, character_id, slot).await?;
        }

        ItemType::ChickenMine => {
            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::UseItem.id()).write_u16(item_id);
            responses.push(writer.into_bytes());
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::Soundmaker => {
            broadcast_soundmaker(server, room_id, item_id, player_id).await;
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::Gum => {
            broadcast_visual_effect(server, room_id, item_id, x, y).await;
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::Soda | ItemType::SpeedSoda | ItemType::JumpSoda => {
            // Sodas - consume (client handles visual/effect)
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::SimpleSeed | ItemType::BlueSeed => {
            // Seeds require special planting logic
            debug!("Seed planting not fully implemented yet");
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::Fairy | ItemType::BluePinwheel | ItemType::RedPinwheel | ItemType::GlowPinwheel => {
            // These require targeting a tree
            debug!("Tree enhancement items not fully implemented yet");
        }

        ItemType::WeakCannonKit => {
            debug!("Cannon building not fully implemented yet");
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        _ => {
            debug!("Item {} cannot be used", item_id);
        }
    }

    Ok(responses)
}

/// Handle Warp-Wing usage - teleport to spawn
async fn handle_warp_wing(
    responses: &mut Vec<Vec<u8>>,
    server: &Arc<Server>,
    session: &Arc<RwLock<PlayerSession>>,
    character_id: i64,
) -> Result<()> {
    let spawn_x = crate::constants::DEFAULT_SPAWN_X;
    let spawn_y = crate::constants::DEFAULT_SPAWN_Y;
    let spawn_room = crate::constants::DEFAULT_SPAWN_ROOM;

    // Send use item response for warp effect
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::UseItem.id())
        .write_u16(1) // item_id for Warp-Wing
        .write_u8(1)  // self = true
        .write_u16(spawn_room)
        .write_u16(spawn_x)
        .write_u16(spawn_y);
    responses.push(writer.into_bytes());

    // Update session
    {
        let mut session_guard = session.write().await;
        session_guard.x = spawn_x;
        session_guard.y = spawn_y;
        session_guard.room_id = spawn_room;
    }

    // Update database
    crate::db::update_position(
        &server.db,
        character_id,
        spawn_x as i16,
        spawn_y as i16,
        spawn_room as i16,
    ).await?;

    // Item already consumed client-side
    Ok(())
}

/// Handle slimebag usage - add points
async fn handle_slimebag(
    item_type: &ItemType,
    server: &Arc<Server>,
    session: &Arc<RwLock<PlayerSession>>,
    character_id: i64,
    slot: u8,
) -> Result<()> {
    let points_to_add: i64 = match item_type {
        ItemType::Slimebag50 => 50,
        ItemType::Slimebag200 => 200,
        ItemType::Slimebag500 => 500,
        _ => 0,
    };

    let account_id = session.read().await.account_id.unwrap_or(0);
    let character = crate::db::find_character_by_account(&server.db, account_id).await?;
    
    if let Some(char) = character {
        let new_points = (char.points + points_to_add).min(crate::constants::MAX_POINTS as i64);
        crate::db::update_points(&server.db, character_id, new_points).await?;
        session.write().await.points = new_points as u32;
    }

    crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
    Ok(())
}

/// Broadcast visual effect to room (smokebomb, applebomb, gum)
/// Includes the sender - they need to receive the message to show the effect
async fn broadcast_visual_effect(server: &Arc<Server>, room_id: u16, item_id: u16, x: u16, y: u16) {
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::UseItem.id())
        .write_u16(item_id)
        .write_u16(x)
        .write_u16(y);
    let msg = writer.into_bytes();
    
    debug!("Broadcasting visual effect for item {} at ({}, {}) to room {}", item_id, x, y, room_id);
    
    let room_players = server.game_state.get_room_players(room_id).await;
    debug!("Room {} has {} players: {:?}", room_id, room_players.len(), room_players);
    
    for player_id in room_players {
        if let Some(session_id) = server.game_state.players_by_id.get(&player_id) {
            if let Some(session) = server.sessions.get(&session_id) {
                debug!("Sending visual effect to player {}", player_id);
                session.write().await.queue_message(msg.clone());
            }
        }
    }
}

/// Broadcast bubbles effect to room
/// Includes the sender - they need to receive the message to show the effect
async fn broadcast_bubbles(server: &Arc<Server>, room_id: u16, item_id: u16, x: u16, y: u16, direction: u8) {
    let amount = 5u8;
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::UseItem.id())
        .write_u16(item_id)
        .write_u16(x)
        .write_u16(y)
        .write_u8(direction)
        .write_u8(amount);
    let msg = writer.into_bytes();
    
    let room_players = server.game_state.get_room_players(room_id).await;
    for player_id in room_players {
        if let Some(session_id) = server.game_state.players_by_id.get(&player_id) {
            if let Some(session) = server.sessions.get(&session_id) {
                session.write().await.queue_message(msg.clone());
            }
        }
    }
}

/// Broadcast soundmaker to room
/// Includes the sender - they need to receive the message to play the sound
async fn broadcast_soundmaker(server: &Arc<Server>, room_id: u16, item_id: u16, user_player_id: u16) {
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::UseItem.id())
        .write_u16(item_id)
        .write_u16(user_player_id);
    let msg = writer.into_bytes();
    
    let room_players = server.game_state.get_room_players(room_id).await;
    for pid in room_players {
        if let Some(session_id) = server.game_state.players_by_id.get(&pid) {
            if let Some(session) = server.sessions.get(&session_id) {
                session.write().await.queue_message(msg.clone());
            }
        }
    }
}
