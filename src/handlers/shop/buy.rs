//! Shop buy handler for Slime Online 2
//!
//! Shop data flow:
//! - Shop item definitions come from `shops.toml` config (cat, item, stock, avail)
//! - Prices come from `prices.toml` config
//! - Slot unlock status can be overridden by `shop_slot_unlocked` table (upgrader system)
//! - Current stock is tracked in `shop_stock` table (runtime state)
//!
//! MSG_SHOP_BUY (28):
//!   Client -> Server: pos_id (1 byte)
//!   Server -> Client (success): category (1) + slot (1) + item_id (2) + price (2)
//!
//! MSG_SHOP_BUY_FAIL (29):
//!   Server -> Client: case (1) + [pos_id (1) if case=1]
//!   case 1 = out of stock, case 2 = not enough points
//!
//! MSG_ROOM_SHOP_INFO (27):
//!   Server -> Client: count (1) + [slot_id (1) if count==1] +
//!                     for each: category (1) + price (2) + stock (1) + item_id (2)

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::config::{GameConfig, ShopSlotConfig};
use crate::db::DbPool;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::rate_limit::ActionType;
use crate::Server;

/// Shop item category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ShopCategory {
    Outfit = 1,
    Item = 2,
    Accessory = 3,
    Tool = 4,
}

impl ShopCategory {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Outfit),
            2 => Some(Self::Item),
            3 => Some(Self::Accessory),
            4 => Some(Self::Tool),
            _ => None,
        }
    }
}

/// Resolved shop item with all data needed for display/purchase
#[derive(Debug, Clone)]
pub struct ResolvedShopItem {
    pub slot_id: u8,
    pub category: u8,
    pub item_id: u16,
    pub price: u32,
    pub stock: u16,         // Current stock (0 = sold out, 65535 = unlimited)
    pub max_stock: u16,     // Maximum stock from config (0 = unlimited)
    pub is_available: bool, // Whether slot is available (config + upgrader)
}

/// Check if a shop slot has been unlocked by the upgrader system
async fn is_slot_unlocked_by_upgrader(pool: &DbPool, room_id: u16, slot_id: u8) -> Option<bool> {
    let result: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT available
        FROM shop_slot_unlocked
        WHERE room_id = ? AND slot_id = ?
        "#,
    )
    .bind(room_id as i64)
    .bind(slot_id as i64)
    .fetch_optional(pool)
    .await
    .ok()?;

    result.map(|(a,)| a == 1)
}

/// Get current stock from runtime state table
async fn get_current_stock(pool: &DbPool, room_id: u16, slot_id: u8) -> Option<u16> {
    let result: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT current_stock
        FROM shop_stock
        WHERE room_id = ? AND slot_id = ?
        "#,
    )
    .bind(room_id as i64)
    .bind(slot_id as i64)
    .fetch_optional(pool)
    .await
    .ok()?;

    result.map(|(s,)| s as u16)
}

/// Update current stock in runtime state table
async fn update_current_stock(
    pool: &DbPool,
    room_id: u16,
    slot_id: u8,
    new_stock: u16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO shop_stock (room_id, slot_id, current_stock, last_purchase)
        VALUES (?, ?, ?, datetime('now'))
        ON CONFLICT (room_id, slot_id) DO UPDATE SET
            current_stock = ?,
            last_purchase = datetime('now')
        "#,
    )
    .bind(room_id as i64)
    .bind(slot_id as i64)
    .bind(new_stock as i64)
    .bind(new_stock as i64)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get the price for an item based on its category
fn get_item_price(config: &GameConfig, category: u8, item_id: u16) -> Option<u32> {
    match category {
        1 => config.prices.get_outfit_price(item_id),
        2 => config.prices.get_item_price(item_id),
        3 => config.prices.get_accessory_price(item_id),
        4 => config.prices.tools.get(&(item_id as u8)).map(|t| t.price),
        _ => None,
    }
}

/// Get the stock bonus for a room (from upgrader investments)
async fn get_room_stock_bonus(pool: &DbPool, room_id: u16) -> u16 {
    crate::db::get_shop_stock_bonus(pool, room_id)
        .await
        .unwrap_or(0)
}

/// Resolve a shop slot from config to a full ResolvedShopItem
/// This combines config data with runtime state from database
async fn resolve_shop_slot(
    pool: &DbPool,
    config: &GameConfig,
    room_id: u16,
    slot_id: u8,
    slot_config: &ShopSlotConfig,
    stock_bonus: u16,
) -> Option<ResolvedShopItem> {
    // Get price from prices.toml
    let price = get_item_price(config, slot_config.cat, slot_config.item)?;

    // Determine if slot is available:
    // 1. Check if upgrader has overridden this slot's availability
    // 2. Fall back to config's avail field
    let is_available = match is_slot_unlocked_by_upgrader(pool, room_id, slot_id).await {
        Some(unlocked) => unlocked,
        None => slot_config.avail,
    };

    // Calculate max stock:
    // - Config stock (0 = unlimited)
    // - Plus stock bonus from upgrader (only applies to limited items)
    let config_max = slot_config.stock;
    let max_stock = if config_max == 0 {
        0 // Unlimited stays unlimited
    } else {
        config_max.saturating_add(stock_bonus)
    };

    // Get current stock:
    // 1. Check runtime state table for current stock
    // 2. Fall back to max_stock (which includes bonus)
    let stock = match get_current_stock(pool, room_id, slot_id).await {
        Some(current) => {
            // If stock bonus increased, we might have more max stock now
            // Current stock should not exceed new max
            current.min(max_stock)
        }
        None => max_stock, // Use max stock as initial stock
    };

    Some(ResolvedShopItem {
        slot_id,
        category: slot_config.cat,
        item_id: slot_config.item,
        price,
        stock,
        max_stock,
        is_available,
    })
}

/// Get all available shop items for a room
/// Returns only items that are available (unlocked) and resolves their current state
pub async fn get_room_shop_items(
    pool: &DbPool,
    config: &GameConfig,
    room_id: u16,
) -> Vec<ResolvedShopItem> {
    let room_config = match config.shops.get_room(room_id) {
        Some(rc) => rc,
        None => return vec![],
    };

    // Get the stock bonus for this room (from upgrader investments)
    let stock_bonus = get_room_stock_bonus(pool, room_id).await;

    let mut items = Vec::new();

    for (idx, slot_config) in room_config.slots.iter().enumerate() {
        let slot_id = (idx + 1) as u8;

        if let Some(resolved) =
            resolve_shop_slot(pool, config, room_id, slot_id, slot_config, stock_bonus).await
        {
            // Only include available slots
            if resolved.is_available {
                items.push(resolved);
            }
        }
    }

    items
}

/// Get a specific shop item by room and slot
/// Returns the item even if not available (for validation purposes)
pub async fn get_shop_item(
    pool: &DbPool,
    config: &GameConfig,
    room_id: u16,
    slot_id: u8,
) -> Option<ResolvedShopItem> {
    let room_config = config.shops.get_room(room_id)?;
    let slot_config = room_config.slots.get((slot_id - 1) as usize)?;

    // Get the stock bonus for this room (from upgrader investments)
    let stock_bonus = get_room_stock_bonus(pool, room_id).await;

    resolve_shop_slot(pool, config, room_id, slot_id, slot_config, stock_bonus).await
}

/// Find an empty slot in player's inventory for the given category
/// Returns slot number (1-9) or None if full
pub fn find_empty_slot(inventory: &crate::db::Inventory, category: ShopCategory) -> Option<u8> {
    match category {
        ShopCategory::Outfit => {
            let outfits = inventory.outfits();
            for (i, &item) in outfits.iter().enumerate() {
                if item == 0 {
                    return Some((i + 1) as u8);
                }
            }
            None
        }
        ShopCategory::Item => {
            let items = inventory.items();
            for (i, &item) in items.iter().enumerate() {
                if item == 0 {
                    return Some((i + 1) as u8);
                }
            }
            None
        }
        ShopCategory::Accessory => {
            let accessories = inventory.accessories();
            for (i, &item) in accessories.iter().enumerate() {
                if item == 0 {
                    return Some((i + 1) as u8);
                }
            }
            None
        }
        ShopCategory::Tool => {
            let tools = inventory.tools();
            for (i, &item) in tools.iter().enumerate() {
                if item == 0 {
                    return Some((i + 1) as u8);
                }
            }
            None
        }
    }
}

/// Handle MSG_SHOP_BUY (28)
pub async fn handle_shop_buy(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let mut reader = MessageReader::new(payload);
    let pos_id = reader.read_u8()?;

    let (character_id, player_id, room_id, session_id, points) = {
        let session_guard = session.read().await;
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }
        (
            session_guard.character_id,
            session_guard.player_id,
            session_guard.room_id,
            session_guard.session_id,
            session_guard.points,
        )
    };

    // Rate limit shop purchases
    if !server
        .rate_limiter
        .check_player(session_id.as_u128() as u64, ActionType::ShopBuy)
        .await
        .is_allowed()
    {
        debug!("Shop buy rate limited");
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

    debug!(
        "Player {} attempting to buy from slot {} in room {}",
        player_id, pos_id, room_id
    );

    // Get the shop item from config + runtime state
    let shop_item = match get_shop_item(&server.db, &server.game_config, room_id, pos_id).await {
        Some(item) => item,
        None => {
            warn!("Shop item not found: room {} slot {}", room_id, pos_id);
            return Ok(vec![]);
        }
    };

    // Check if slot is available (not locked by upgrader)
    if !shop_item.is_available {
        warn!("Shop slot not available: room {} slot {}", room_id, pos_id);
        return Ok(vec![]);
    }

    // Check if item is in stock (stock=0 means sold out, but max_stock=0 means unlimited)
    let is_unlimited = shop_item.max_stock == 0;
    if !is_unlimited && shop_item.stock == 0 {
        info!("Item out of stock: room {} slot {}", room_id, pos_id);
        let mut writer = MessageWriter::new();
        writer
            .write_u16(MessageType::ShopBuyFail.id())
            .write_u8(1) // case 1 = out of stock
            .write_u8(pos_id);
        return Ok(vec![writer.into_bytes()]);
    }

    // Check if player has enough points
    if points < shop_item.price {
        info!(
            "Player {} doesn't have enough points: has {}, needs {}",
            player_id, points, shop_item.price
        );
        let mut writer = MessageWriter::new();
        writer.write_u16(MessageType::ShopBuyFail.id()).write_u8(2); // case 2 = not enough points
        return Ok(vec![writer.into_bytes()]);
    }

    // Get player's inventory
    let inventory = match crate::db::get_inventory(&server.db, character_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };

    // Parse category
    let category = match ShopCategory::from_u8(shop_item.category) {
        Some(cat) => cat,
        None => {
            warn!("Invalid shop category: {}", shop_item.category);
            return Ok(vec![]);
        }
    };

    // Find empty slot in player's inventory
    let inv_slot = match find_empty_slot(&inventory, category) {
        Some(s) => s,
        None => {
            // Inventory full - client should check this, but reject anyway
            debug!(
                "Player {} inventory full for category {:?}",
                player_id, category
            );
            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::ShopBuyFail.id()).write_u8(2); // Use case 2 for inventory full
            return Ok(vec![writer.into_bytes()]);
        }
    };

    // All checks passed - process the purchase

    // 1. Add item to player's inventory
    match category {
        ShopCategory::Outfit => {
            crate::db::update_outfit_slot(
                &server.db,
                character_id,
                inv_slot,
                shop_item.item_id as i16,
            )
            .await?;
        }
        ShopCategory::Item => {
            crate::db::update_item_slot(
                &server.db,
                character_id,
                inv_slot,
                shop_item.item_id as i16,
            )
            .await?;
        }
        ShopCategory::Accessory => {
            crate::db::update_accessory_slot(
                &server.db,
                character_id,
                inv_slot,
                shop_item.item_id as i16,
            )
            .await?;
        }
        ShopCategory::Tool => {
            crate::db::update_tool_slot(
                &server.db,
                character_id,
                inv_slot,
                shop_item.item_id as i16,
            )
            .await?;
        }
    }

    // 2. Deduct points
    let new_points = points - shop_item.price;
    crate::db::update_points(&server.db, character_id, new_points as i64).await?;

    // 3. Update session points
    {
        let mut session_guard = session.write().await;
        session_guard.points = new_points;
    }

    // 4. If limited item (max_stock > 0), decrement stock
    if !is_unlimited && shop_item.stock > 0 {
        let new_stock = shop_item.stock - 1;
        update_current_stock(&server.db, room_id, pos_id, new_stock).await?;

        // If now sold out, broadcast to all players in room
        if new_stock == 0 {
            let room_players = server.game_state.get_room_players(room_id).await;
            for other_player_id in room_players {
                if other_player_id == player_id {
                    continue; // Skip buyer, they get ShopBuy response
                }
                if let Some(other_session_id) =
                    server.game_state.players_by_id.get(&other_player_id)
                {
                    if let Some(other_handle) = server.sessions.get(&other_session_id) {
                        let mut writer = MessageWriter::new();
                        writer
                            .write_u16(MessageType::ShopStock.id())
                            .write_u8(1) // case 1 = sold out
                            .write_u8(pos_id);
                        other_handle.queue_message(writer.into_bytes()).await;
                    }
                }
            }
        }
    }

    info!(
        "Player {} bought {:?} {} for {} points (inv slot {})",
        player_id, category, shop_item.item_id, shop_item.price, inv_slot
    );

    // Send success response
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::ShopBuy.id())
        .write_u8(shop_item.category)
        .write_u8(inv_slot)
        .write_u16(shop_item.item_id)
        .write_u16(shop_item.price as u16);

    Ok(vec![writer.into_bytes()])
}

/// Build MSG_ROOM_SHOP_INFO (27) message for a room with shops
/// Returns None if there are no shops in the room
pub async fn build_room_shop_info(server: &Arc<Server>, room_id: u16) -> Result<Option<Vec<u8>>> {
    // Get available shop items for this room (filtered by availability)
    let shop_items = get_room_shop_items(&server.db, &server.game_config, room_id).await;

    if shop_items.is_empty() {
        return Ok(None); // No shop in this room (or all slots locked)
    }

    let count = shop_items.len() as u8;

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::RoomShopInfo.id())
        .write_u8(count);

    if count == 1 {
        // Special case: single item includes slot_id first
        let item = &shop_items[0];
        // Stock byte: 0 = sold out, 1+ = in stock
        // Original server sends 1 for "in stock", 0 for "sold out"
        // Unlimited items (max_stock=0) are always in stock
        let stock_value = if item.max_stock == 0 {
            1u8 // Unlimited = always in stock
        } else if item.stock == 0 {
            0u8 // Sold out
        } else {
            1u8 // In stock
        };
        writer
            .write_u8(item.slot_id)
            .write_u8(item.category)
            .write_u16(item.price as u16)
            .write_u8(stock_value)
            .write_u16(item.item_id);
    } else {
        // Multiple items: slot is implicit (1, 2, 3, ...)
        for item in &shop_items {
            let stock_value = if item.max_stock == 0 {
                1u8 // Unlimited = always in stock
            } else if item.stock == 0 {
                0u8 // Sold out
            } else {
                1u8 // In stock
            };
            debug!(
                "  Shop slot {}: cat={}, price={}, stock={}, item_id={}",
                item.slot_id, item.category, item.price, stock_value, item.item_id
            );
            writer
                .write_u8(item.category)
                .write_u16(item.price as u16)
                .write_u8(stock_value)
                .write_u16(item.item_id);
        }
    }

    debug!(
        "Built shop info for room {} ({} items), msg bytes: {:02X?}",
        room_id,
        count,
        writer.as_bytes()
    );

    Ok(Some(writer.into_bytes()))
}

/// Restock all shops (called daily or on demand)
/// Resets current_stock to max_stock for all items with limited stock
pub async fn restock_all_shops(pool: &DbPool) -> Result<(), sqlx::Error> {
    // Delete all entries from shop_stock table
    // This effectively resets stock to config values (which are used as defaults)
    sqlx::query("DELETE FROM shop_stock").execute(pool).await?;

    info!("All shop stock has been reset");
    Ok(())
}

/// Restock a specific room's shop
pub async fn restock_room_shop(pool: &DbPool, room_id: u16) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM shop_stock WHERE room_id = ?")
        .bind(room_id as i64)
        .execute(pool)
        .await?;

    info!("Shop stock for room {} has been reset", room_id);
    Ok(())
}
