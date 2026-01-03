//! Vending machine handlers
//!
//! Handles vending machine purchases:
//! - MSG_BUY_GUM (113) - Buy gum from gum machine
//! - MSG_BUY_SODA (114) - Buy soda from soda machine

use std::sync::Arc;

use anyhow::Result;
use rand::Rng;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::db;
use crate::game::PlayerSession;
use crate::protocol::{MessageType, MessageWriter};
use crate::Server;

/// Gum price in points
const PRICE_GUM: u32 = 20;

/// Soda price in points
const PRICE_SODA: u32 = 50;

/// Regular gum item IDs (27-32)
const GUM_ITEMS: [u16; 6] = [27, 28, 29, 30, 31, 32];

/// Lucky coin item ID (rare from gum machine)
const LUCKY_COIN: u16 = 33;

/// Lucky coin chance (1 in N)
const LUCKY_COIN_CHANCE: u32 = 100;

/// Soda item IDs (34, 35, 36)
const SODA_ITEMS: [u16; 3] = [34, 35, 36];

/// Valid vending machine rooms
const VENDING_ROOMS: [u16; 2] = [42, 121]; // New City, Old City

/// Handle MSG_BUY_GUM (113) - Buy gum from gum machine
pub async fn handle_buy_gum(
    _payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let (char_id, room_id, points) = {
        let session_guard = session.read().await;
        (
            session_guard.character_id,
            session_guard.room_id,
            session_guard.points,
        )
    };

    let char_id = match char_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    debug!("Buy gum: char_id={}, room={}, points={}", char_id, room_id, points);

    // Check if player is in valid room
    if !VENDING_ROOMS.contains(&room_id) {
        warn!("Player tried to buy gum in invalid room {}", room_id);
        return Ok(vec![]);
    }

    // Check if player has enough points
    if points < PRICE_GUM {
        warn!("Player doesn't have enough points for gum: {} < {}", points, PRICE_GUM);
        return Ok(vec![]);
    }

    // Load inventory to find free slot
    let inventory = match db::get_inventory(&server.db, char_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };

    let items = inventory.items();
    let free_slot = items.iter().position(|&x| x == 0);
    let free_slot = match free_slot {
        Some(idx) => idx,
        None => {
            warn!("Player has no free item slot for gum");
            return Ok(vec![]);
        }
    };

    // Determine item (rare chance for lucky coin)
    // Note: Generate random values and drop rng immediately before any await
    let item_id = {
        let mut rng = rand::thread_rng();
        if rng.gen_range(0..LUCKY_COIN_CHANCE) == 0 {
            LUCKY_COIN
        } else {
            GUM_ITEMS[rng.gen_range(0..GUM_ITEMS.len())]
        }
    };

    // Deduct points
    let new_points = points - PRICE_GUM;
    {
        let mut session_guard = session.write().await;
        session_guard.points = new_points;
    }
    let _ = db::update_points(&server.db, char_id, new_points as i64).await;

    // Add item to inventory
    let mut new_items = items;
    new_items[free_slot] = item_id;
    let _ = db::update_inventory_items(&server.db, char_id, &new_items).await;

    info!(
        "Player {} bought gum {} (slot {}) for {} points",
        char_id, item_id, free_slot + 1, PRICE_GUM
    );

    // Send response
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BuyGum.id());
    writer.write_u8((free_slot + 1) as u8);
    writer.write_u16(item_id);

    Ok(vec![writer.into_bytes()])
}

/// Handle MSG_BUY_SODA (114) - Buy soda from soda machine
pub async fn handle_buy_soda(
    _payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let (char_id, room_id, points) = {
        let session_guard = session.read().await;
        (
            session_guard.character_id,
            session_guard.room_id,
            session_guard.points,
        )
    };

    let char_id = match char_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    debug!("Buy soda: char_id={}, room={}, points={}", char_id, room_id, points);

    // Check if player is in valid room
    if !VENDING_ROOMS.contains(&room_id) {
        warn!("Player tried to buy soda in invalid room {}", room_id);
        return Ok(vec![]);
    }

    // Check if player has enough points
    if points < PRICE_SODA {
        warn!("Player doesn't have enough points for soda: {} < {}", points, PRICE_SODA);
        return Ok(vec![]);
    }

    // Load inventory to find free slot
    let inventory = match db::get_inventory(&server.db, char_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };

    let items = inventory.items();
    let free_slot = items.iter().position(|&x| x == 0);
    let free_slot = match free_slot {
        Some(idx) => idx,
        None => {
            warn!("Player has no free item slot for soda");
            return Ok(vec![]);
        }
    };

    // Randomly select a soda
    // Note: Generate random values and drop rng immediately before any await
    let item_id = {
        let mut rng = rand::thread_rng();
        SODA_ITEMS[rng.gen_range(0..SODA_ITEMS.len())]
    };

    // Deduct points
    let new_points = points - PRICE_SODA;
    {
        let mut session_guard = session.write().await;
        session_guard.points = new_points;
    }
    let _ = db::update_points(&server.db, char_id, new_points as i64).await;

    // Add item to inventory
    let mut new_items = items;
    new_items[free_slot] = item_id;
    let _ = db::update_inventory_items(&server.db, char_id, &new_items).await;

    info!(
        "Player {} bought soda {} (slot {}) for {} points",
        char_id, item_id, free_slot + 1, PRICE_SODA
    );

    // Send response
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BuySoda.id());
    writer.write_u8((free_slot + 1) as u8);
    writer.write_u16(item_id);

    Ok(vec![writer.into_bytes()])
}
