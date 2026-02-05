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

use crate::Server;
use crate::anticheat::{validate_position_bounds, HackType};
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::rate_limit::ActionType;
use crate::validation::{validate_item_slot, handle_points_overflow};

use super::database::{ItemType, get_item_info};

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

    // Get session info first (needed for hack reporting)
    let (character_id, player_id, room_id, session_id, session_x, session_y, account_id, ip_address) = {
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
            session_guard.account_id,
            session_guard.ip_address.clone(),
        )
    };

    // Validate slot
    if let Err(e) = validate_item_slot(slot) {
        // Report hack - invalid slot is a protocol violation
        if let Some(acc_id) = account_id {
            let _ = server
                .anticheat
                .report_hack(
                    &server.db,
                    acc_id,
                    character_id,
                    HackType::InvalidSlot,
                    &format!("Invalid item slot: {} - {}", slot, e.message),
                    Some(&ip_address),
                    None,
                    &server.game_config.game.anticheat,
                )
                .await;
        }
        warn!("HACK: Invalid item slot {}: {}", slot, e.message);
        return Ok(vec![]);
    }

    // Rate limit item usage
    if !server
        .rate_limiter
        .check_player(session_id.as_u128() as u64, ActionType::UseItem)
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
        // Report hack - trying to use item from empty slot
        if let Some(acc_id) = account_id {
            let _ = server
                .anticheat
                .report_hack(
                    &server.db,
                    acc_id,
                    Some(character_id),
                    HackType::ItemNotInSlot,
                    &format!("Tried to use item from empty slot {}", slot),
                    Some(&ip_address),
                    None,
                    &server.game_config.game.anticheat,
                )
                .await;
        }
        warn!("HACK: Slot {} is empty", slot);
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
                warn!(
                    "Invalid item use position: ({}, {}), using session position",
                    ux, uy
                );
                (session_x, session_y)
            }
        }
        _ => (session_x, session_y),
    };

    // Handle item effects based on type
    match item_info.item_type {
        ItemType::WarpWing => {
            handle_warp_wing(&mut responses, server, &session, character_id, slot, x, y).await?;
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
            writer
                .write_u16(MessageType::UseItem.id())
                .write_u16(item_id);
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

        ItemType::Fairy
        | ItemType::BluePinwheel
        | ItemType::RedPinwheel
        | ItemType::GlowPinwheel => {
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

/// Handle Warp-Wing (Fly Wing) usage - teleport to last save point
///
/// The original server reads the player's saved position from their account file.
/// In our implementation, this is the character's stored x/y/room_id in the database.
async fn handle_warp_wing(
    responses: &mut Vec<Vec<u8>>,
    server: &Arc<Server>,
    session: &Arc<RwLock<PlayerSession>>,
    character_id: i64,
    slot: u8,
    current_x: u16,
    current_y: u16,
) -> Result<()> {
    // Get character's saved position from database (last save point)
    let character = crate::db::find_character_by_id(&server.db, character_id).await?;

    let (save_x, save_y, save_room) = match character {
        Some(char) => (char.x as u16, char.y as u16, char.room_id as u16),
        None => {
            // Fallback to current position if character not found
            let session_guard = session.read().await;
            (session_guard.x, session_guard.y, session_guard.room_id)
        }
    };

    // Send use item response to self (case 1 = self)
    // Format: msg_type(2) + item_id(2) + self_flag(1) + room(2) + x(2) + y(2)
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::UseItem.id())
        .write_u16(1) // item_id for Fly Wing
        .write_u8(1) // self = true
        .write_u16(save_room)
        .write_u16(save_x)
        .write_u16(save_y);
    responses.push(writer.into_bytes());

    // Broadcast to same room (case 0 = others see the warp effect)
    // Format: msg_type(2) + item_id(2) + self_flag(0) + x(2) + y(2)
    let room_id = session.read().await.room_id;
    let player_id = session.read().await.player_id;

    let mut broadcast_writer = MessageWriter::new();
    broadcast_writer
        .write_u16(MessageType::UseItem.id())
        .write_u16(1) // item_id for Fly Wing
        .write_u8(0) // self = false (for others)
        .write_u16(current_x)
        .write_u16(current_y);
    let broadcast_msg = broadcast_writer.into_bytes();

    // Send to other players in the same room
    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if Some(other_player_id) == player_id {
            continue; // Skip self
        }
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id)
            && let Some(other_handle) = server.sessions.get(&other_session_id)
        {
            other_handle.queue_message(broadcast_msg.clone()).await;
        }
    }

    // Update session with saved position
    {
        let mut session_guard = session.write().await;
        session_guard.x = save_x;
        session_guard.y = save_y;
        session_guard.room_id = save_room;
    }

    // Consume the item
    crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;

    Ok(())
}

/// Handle slimebag usage - add points with overflow to bank
async fn handle_slimebag(
    item_type: &ItemType,
    server: &Arc<Server>,
    session: &Arc<RwLock<PlayerSession>>,
    character_id: i64,
    slot: u8,
) -> Result<()> {
    let points_to_add: u32 = match item_type {
        ItemType::Slimebag50 => 50,
        ItemType::Slimebag200 => 200,
        ItemType::Slimebag500 => 500,
        _ => 0,
    };

    let current_points = session.read().await.points;
    let max_points = server.game_config.game.limits.max_points;

    // Use overflow handling - excess goes to bank automatically
    let overflow_result = handle_points_overflow(current_points, points_to_add, max_points);

    // Update points in database
    crate::db::update_points(&server.db, character_id, overflow_result.new_points as i64).await?;
    session.write().await.points = overflow_result.new_points;

    // If there's overflow, deposit to bank
    if overflow_result.to_bank > 0 {
        let current_bank = crate::db::get_bank_balance(&server.db, character_id)
            .await
            .unwrap_or(0);
        let new_bank = current_bank + overflow_result.to_bank as i64;
        if let Err(e) = crate::db::update_bank_balance(&server.db, character_id, new_bank).await {
            warn!("Failed to deposit slimebag overflow to bank: {}", e);
        } else {
            debug!(
                "Slimebag overflow: deposited {} to bank (new balance: {})",
                overflow_result.to_bank, new_bank
            );
        }
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

    debug!(
        "Broadcasting visual effect for item {} at ({}, {}) to room {}",
        item_id, x, y, room_id
    );

    let room_players = server.game_state.get_room_players(room_id).await;
    debug!(
        "Room {} has {} players: {:?}",
        room_id,
        room_players.len(),
        room_players
    );

    for player_id in room_players {
        if let Some(session_id) = server.game_state.players_by_id.get(&player_id)
            && let Some(handle) = server.sessions.get(&session_id)
        {
            debug!("Sending visual effect to player {}", player_id);
            handle.queue_message(msg.clone()).await;
        }
    }
}

/// Broadcast bubbles effect to room
/// Includes the sender - they need to receive the message to show the effect
async fn broadcast_bubbles(
    server: &Arc<Server>,
    room_id: u16,
    item_id: u16,
    x: u16,
    y: u16,
    direction: u8,
) {
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
        if let Some(session_id) = server.game_state.players_by_id.get(&player_id)
            && let Some(handle) = server.sessions.get(&session_id)
        {
            handle.queue_message(msg.clone()).await;
        }
    }
}

/// Broadcast soundmaker to room
/// Includes the sender - they need to receive the message to play the sound
async fn broadcast_soundmaker(
    server: &Arc<Server>,
    room_id: u16,
    item_id: u16,
    user_player_id: u16,
) {
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::UseItem.id())
        .write_u16(item_id)
        .write_u16(user_player_id);
    let msg = writer.into_bytes();

    let room_players = server.game_state.get_room_players(room_id).await;
    for pid in room_players {
        if let Some(session_id) = server.game_state.players_by_id.get(&pid)
            && let Some(handle) = server.sessions.get(&session_id)
        {
            handle.queue_message(msg.clone()).await;
        }
    }
}
