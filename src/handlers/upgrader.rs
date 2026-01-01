//! Upgrader system handlers - community investment upgrades
//!
//! The upgrader allows players to collectively invest points to unlock
//! new features for their town (new shop items, warp destinations, etc.)

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::db;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

/// Category names matching the config file
const CATEGORY_NAMES: [&str; 5] = [
    "Other",
    "Warp Center",
    "Outfit Shop",
    "Acs Shop",
    "Item Shop",
];

/// Convert category ID to category name
fn category_name(cat_id: u8) -> Option<&'static str> {
    CATEGORY_NAMES.get(cat_id as usize).copied()
}

/// Handle MSG_UPGRADER_GET (108)
/// Client requests upgrade slot information for a page
pub async fn handle_upgrader_get(
    payload: &[u8],
    server: &Arc<Server>,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);

    let town_id = reader.read_u16()?;
    let category_id = reader.read_u8()?;
    let page = reader.read_u8()?;

    let category = match category_name(category_id) {
        Some(name) => name,
        None => {
            warn!("Invalid upgrader category: {}", category_id);
            return Ok(vec![]);
        }
    };

    debug!(
        "Upgrader GET: town={}, category={}, page={}",
        town_id, category, page
    );

    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::UpgraderGet.id());

    // Get town config from game_config
    let town_config = server.game_config.upgrader.get_town(town_id);

    // Process 4 slots for this page
    for slot_offset in 0..4u8 {
        let slot_id = (page * 4) + slot_offset + 1;

        if let Some(town) = town_config {
            if let Some(upgrade) = town.upgrades.get(&(category.to_string(), slot_id)) {
                // Check if slot is unlocked (visible to players)
                let is_unlocked =
                    match db::is_slot_unlocked(&server.db, town_id, category, slot_id).await {
                        Ok(Some(unlocked)) => unlocked,
                        Ok(None) => upgrade.unlocked, // Use config default if not in DB
                        Err(_) => upgrade.unlocked,
                    };

                if is_unlocked {
                    // Get paid amount from database
                    let paid = db::get_paid_amount(&server.db, town_id, category, slot_id)
                        .await
                        .unwrap_or(0);

                    let need = upgrade.need;

                    // Calculate percentage
                    let percentage = if paid >= need {
                        100u8
                    } else if need == 0 {
                        0u8
                    } else {
                        let pct = ((paid as f64 / need as f64) * 100.0).round() as u8;
                        // Don't show 100% unless actually complete
                        if pct >= 100 {
                            99
                        } else {
                            pct
                        }
                    };

                    writer.write_string(&upgrade.name);
                    writer.write_u8(percentage);
                } else {
                    // Slot exists but is locked - send special value 250
                    writer.write_string("");
                    writer.write_u8(250);
                }
            } else {
                // Slot doesn't exist
                writer.write_string("");
                writer.write_u8(0);
            }
        } else {
            // Town not configured
            writer.write_string("");
            writer.write_u8(0);
        }
    }

    // Check if there are more slots after this page
    let has_more = if let Some(town) = town_config {
        let next_slot = (page * 4) + 5;
        if let Some(upgrade) = town.upgrades.get(&(category.to_string(), next_slot)) {
            // Check if the next slot is unlocked
            match db::is_slot_unlocked(&server.db, town_id, category, next_slot).await {
                Ok(Some(unlocked)) => unlocked,
                Ok(None) => upgrade.unlocked,
                Err(_) => upgrade.unlocked,
            }
        } else {
            false
        }
    } else {
        false
    };

    writer.write_u8(if has_more { 1 } else { 0 });

    Ok(vec![writer.into_bytes()])
}

/// Handle MSG_UPGRADER_POINTS (109)
/// Client requests how many points are still needed for a slot
pub async fn handle_upgrader_points(
    payload: &[u8],
    server: &Arc<Server>,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);

    let town_id = reader.read_u16()?;
    let category_id = reader.read_u8()?;
    let slot_id = reader.read_u8()?;

    let category = match category_name(category_id) {
        Some(name) => name,
        None => {
            warn!("Invalid upgrader category: {}", category_id);
            return Ok(vec![]);
        }
    };

    debug!(
        "Upgrader POINTS: town={}, category={}, slot={}",
        town_id, category, slot_id
    );

    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::UpgraderPoints.id());

    // Get upgrade config from game_config
    let upgrade = server
        .game_config
        .upgrader
        .get_upgrade(town_id, category, slot_id);

    if let Some(upgrade) = upgrade {
        // Get paid amount from database
        let paid = db::get_paid_amount(&server.db, town_id, category, slot_id)
            .await
            .unwrap_or(0);

        let remaining = upgrade.need.saturating_sub(paid);

        // Server sends remaining / 10, client multiplies by 10
        writer.write_u16((remaining / 10) as u16);
    } else {
        // Slot doesn't exist - send error code 123
        writer.write_u8(123);
    }

    Ok(vec![writer.into_bytes()])
}

/// Handle MSG_UPGRADER_INVEST (110)
/// Client wants to invest points into an upgrade
pub async fn handle_upgrader_invest(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);

    let town_id = reader.read_u16()?;
    let category_id = reader.read_u8()?;
    let slot_id = reader.read_u8()?;
    let invest_code = reader.read_u8()?;

    let category = match category_name(category_id) {
        Some(name) => name,
        None => {
            warn!("Invalid upgrader category: {}", category_id);
            return Ok(vec![]);
        }
    };

    // Convert invest code to actual points
    let invest_amount: u32 = match invest_code {
        1 => 100,
        2 => 500,
        3 => 1000,
        4 => 5000,
        _ => {
            warn!("Invalid invest code: {}", invest_code);
            return Ok(vec![]);
        }
    };

    debug!(
        "Upgrader INVEST: town={}, category={}, slot={}, amount={}",
        town_id, category, slot_id, invest_amount
    );

    // Get player's points
    let (current_points, char_id) = {
        let session_guard = session.read().await;
        (session_guard.points, session_guard.character_id)
    };

    let char_id = match char_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Validate player has enough points
    if current_points < invest_amount {
        warn!(
            "Player {} tried to invest {} but only has {} points",
            char_id, invest_amount, current_points
        );
        // Just send invest response to reset UI
        let mut writer = MessageWriter::new();
        writer.write_u16(MessageType::UpgraderInvest.id());
        return Ok(vec![writer.into_bytes()]);
    }

    // Get upgrade config from game_config
    let upgrade = match server
        .game_config
        .upgrader
        .get_upgrade(town_id, category, slot_id)
    {
        Some(u) => u.clone(),
        None => {
            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::UpgraderInvest.id());
            return Ok(vec![writer.into_bytes()]);
        }
    };

    // Add investment to database
    let new_paid = db::add_investment(&server.db, town_id, category, slot_id, invest_amount)
        .await
        .unwrap_or(0);

    // Deduct points from player
    let new_points = current_points - invest_amount;
    {
        let mut session_guard = session.write().await;
        session_guard.points = new_points;
    }

    // Update database - use update_points function
    if let Err(e) = db::update_points(&server.db, char_id, new_points as i64).await {
        warn!("Failed to update character points: {}", e);
    }

    let mut responses = Vec::new();

    // Send points decrease message
    let mut points_writer = MessageWriter::new();
    points_writer.write_u16(MessageType::PointsDec.id());
    points_writer.write_u16(invest_amount as u16);
    responses.push(points_writer.into_bytes());

    // Check if upgrade is now complete
    if new_paid >= upgrade.need {
        info!(
            "Upgrade completed! town={}, category={}, slot={}",
            town_id, category, slot_id
        );

        // Apply the upgrade effect
        apply_upgrade_effect(server, town_id, category, slot_id, &upgrade).await;

        // Process unlock chain
        for chain_entry in &upgrade.unlock_chain {
            if let Err(e) = db::set_slot_unlocked(
                &server.db,
                town_id,
                &chain_entry.category,
                chain_entry.slot,
                true,
            )
            .await
            {
                warn!("Failed to unlock chained slot: {}", e);
            } else {
                info!(
                    "Unlocked chained slot: category={}, slot={}",
                    chain_entry.category, chain_entry.slot
                );
            }
        }
    }

    // Send invest response (signals completion, client will refresh)
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::UpgraderInvest.id());
    responses.push(writer.into_bytes());

    Ok(responses)
}

/// Apply the effect of a completed upgrade
async fn apply_upgrade_effect(
    server: &Arc<Server>,
    town_id: u16,
    category: &str,
    _slot_id: u8,
    upgrade: &crate::config::UpgradeSlotConfig,
) {
    match category {
        "Other" => {
            match upgrade.option {
                1 => {
                    // Unlock an unlockable object
                    let room_id = upgrade.other1 as u16;
                    let unlockable_id = upgrade.other2 as u8;

                    if let Err(e) =
                        db::set_unlockable_available(&server.db, room_id, unlockable_id, true).await
                    {
                        warn!("Failed to set unlockable available: {}", e);
                    } else {
                        info!("Unlocked object {} in room {}", unlockable_id, room_id);

                        // Broadcast to players in room
                        broadcast_unlockable_exists(server, room_id, unlockable_id).await;
                    }
                }
                2 => {
                    // Unlock music for Music Changer
                    let room_id = upgrade.other1 as u16;
                    let song_slot = upgrade.other2 as u8;
                    let is_day = upgrade.other3 == 1;

                    if let Err(e) =
                        db::set_music_unlocked(&server.db, room_id, song_slot, is_day).await
                    {
                        warn!("Failed to set music unlocked: {}", e);
                    } else {
                        info!(
                            "Unlocked {} music slot {} in room {}",
                            if is_day { "day" } else { "night" },
                            song_slot,
                            room_id
                        );
                    }
                }
                _ => {
                    warn!("Unknown Other upgrade option: {}", upgrade.option);
                }
            }
        }
        "Warp Center" => {
            // Unlock warp destination
            let town_config = server.game_config.upgrader.get_town(town_id);
            if let Some(town) = town_config {
                let warp_room = town.warp_center_room;
                let warp_slot = upgrade.warp_slot;
                let warp_category = upgrade.warp_category;

                if let Err(e) =
                    db::set_warp_unlocked(&server.db, warp_room, warp_slot, warp_category).await
                {
                    warn!("Failed to set warp unlocked: {}", e);
                } else {
                    info!(
                        "Unlocked warp slot {} category {} in room {}",
                        warp_slot, warp_category, warp_room
                    );
                }
            }
        }
        "Outfit Shop" | "Acs Shop" | "Item Shop" => {
            match upgrade.option {
                1 => {
                    // Unlock new shop slot
                    let shop_room = upgrade.other1 as u16;
                    let shop_slot = upgrade.other2 as u8;

                    if let Err(e) =
                        db::set_shop_slot_unlocked(&server.db, shop_room, shop_slot).await
                    {
                        warn!("Failed to set shop slot unlocked: {}", e);
                    } else {
                        info!("Unlocked shop slot {} in room {}", shop_slot, shop_room);

                        // Broadcast to players in the shop room
                        broadcast_shop_stock_update(server, shop_room).await;
                    }
                }
                2 => {
                    // Increase shop stock bonus (permanent max stock increase)
                    let shop_room = upgrade.other1 as u16;
                    let increase = upgrade.other2 as u16;

                    if let Err(e) =
                        db::increase_shop_stock_bonus(&server.db, shop_room, increase).await
                    {
                        warn!("Failed to increase shop stock bonus: {}", e);
                    } else {
                        info!(
                            "Increased shop stock bonus in room {} by {}",
                            shop_room, increase
                        );

                        // Broadcast to players in the shop room
                        broadcast_shop_stock_update(server, shop_room).await;
                    }
                }
                _ => {
                    warn!("Unknown shop upgrade option: {}", upgrade.option);
                }
            }
        }
        _ => {
            warn!("Unknown upgrade category: {}", category);
        }
    }
}

/// Broadcast MSG_UNLOCKABLE_EXISTS to all players in a room
async fn broadcast_unlockable_exists(server: &Arc<Server>, room_id: u16, unlockable_id: u8) {
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::UnlockableExists.id());
    writer.write_u8(unlockable_id);
    let message = writer.into_bytes();

    // Get all players in the room using game_state
    let room_players = server.game_state.get_room_players(room_id).await;
    for player_id in room_players {
        if let Some(session_id) = server.game_state.players_by_id.get(&player_id) {
            if let Some(session) = server.sessions.get(&session_id) {
                session.write().await.queue_message(message.clone());
            }
        }
    }
}

/// Broadcast MSG_SHOP_STOCK to all players in a room
async fn broadcast_shop_stock_update(server: &Arc<Server>, room_id: u16) {
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::ShopStock.id());
    writer.write_u8(2); // Indicates stock was refreshed
    let message = writer.into_bytes();

    // Get all players in the room using game_state
    let room_players = server.game_state.get_room_players(room_id).await;
    for player_id in room_players {
        if let Some(session_id) = server.game_state.players_by_id.get(&player_id) {
            if let Some(session) = server.sessions.get(&session_id) {
                session.write().await.queue_message(message.clone());
            }
        }
    }
}

/// Send unlockable exists messages when a player enters a room
/// Call this from the warp/room change handler
pub async fn send_room_unlockables(server: &Arc<Server>, room_id: u16) -> Vec<Vec<u8>> {
    let mut responses = Vec::new();

    // Get available unlockables in this room
    let unlockables = db::get_room_unlockables(&server.db, room_id)
        .await
        .unwrap_or_default();

    for unlockable_id in unlockables {
        let mut writer = MessageWriter::new();
        writer.write_u16(MessageType::UnlockableExists.id());
        writer.write_u8(unlockable_id);
        responses.push(writer.into_bytes());
    }

    responses
}
