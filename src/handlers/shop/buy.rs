//! Shop buy handler for Slime Online 2
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
use sqlx::FromRow;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::db::DbPool;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
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

/// Shop item from database
#[derive(Debug, Clone, FromRow)]
pub struct ShopItem {
    pub id: i64,
    pub room_id: i64,
    pub slot_id: i64,
    pub category: i64,
    pub item_id: i64,
    pub price: i64,
    pub stock: i64,
    pub is_limited: i64,
}

/// Get all shop items for a room
pub async fn get_room_shop_items(pool: &DbPool, room_id: u16) -> Result<Vec<ShopItem>, sqlx::Error> {
    sqlx::query_as::<_, ShopItem>(
        r#"
        SELECT id, room_id, slot_id, category, item_id, price, stock, is_limited
        FROM shop_items
        WHERE room_id = ?
        ORDER BY slot_id
        "#,
    )
    .bind(room_id as i64)
    .fetch_all(pool)
    .await
}

/// Get a specific shop item by room and slot
pub async fn get_shop_item(
    pool: &DbPool,
    room_id: u16,
    slot_id: u8,
) -> Result<Option<ShopItem>, sqlx::Error> {
    sqlx::query_as::<_, ShopItem>(
        r#"
        SELECT id, room_id, slot_id, category, item_id, price, stock, is_limited
        FROM shop_items
        WHERE room_id = ? AND slot_id = ?
        "#,
    )
    .bind(room_id as i64)
    .bind(slot_id as i64)
    .fetch_optional(pool)
    .await
}

/// Update shop item stock (for limited items)
pub async fn update_shop_stock(
    pool: &DbPool,
    room_id: u16,
    slot_id: u8,
    stock: u8,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE shop_items
        SET stock = ?, updated_at = datetime('now')
        WHERE room_id = ? AND slot_id = ?
        "#,
    )
    .bind(stock as i64)
    .bind(room_id as i64)
    .bind(slot_id as i64)
    .execute(pool)
    .await?;
    Ok(())
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

    let (character_id, player_id, room_id, points) = {
        let session_guard = session.read().await;
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }
        (
            session_guard.character_id,
            session_guard.player_id,
            session_guard.room_id,
            session_guard.points,
        )
    };

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

    // Get the shop item
    let shop_item = match get_shop_item(&server.db, room_id, pos_id).await? {
        Some(item) => item,
        None => {
            warn!(
                "Shop item not found: room {} slot {}",
                room_id, pos_id
            );
            return Ok(vec![]);
        }
    };

    // Check if item is in stock
    if shop_item.stock == 0 {
        info!("Item out of stock: room {} slot {}", room_id, pos_id);
        let mut writer = MessageWriter::new();
        writer
            .write_u16(MessageType::ShopBuyFail.id())
            .write_u8(1) // case 1 = out of stock
            .write_u8(pos_id);
        return Ok(vec![writer.into_bytes()]);
    }

    // Check if player has enough points
    if (points as i64) < shop_item.price {
        info!(
            "Player {} doesn't have enough points: has {}, needs {}",
            player_id, points, shop_item.price
        );
        let mut writer = MessageWriter::new();
        writer
            .write_u16(MessageType::ShopBuyFail.id())
            .write_u8(2); // case 2 = not enough points
        return Ok(vec![writer.into_bytes()]);
    }

    // Get player's inventory
    let inventory = match crate::db::get_inventory(&server.db, character_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };

    // Parse category
    let category = match ShopCategory::from_u8(shop_item.category as u8) {
        Some(cat) => cat,
        None => {
            warn!("Invalid shop category: {}", shop_item.category);
            return Ok(vec![]);
        }
    };

    // Find empty slot in player's inventory
    let slot = match find_empty_slot(&inventory, category) {
        Some(s) => s,
        None => {
            // Inventory full - client should check this, but reject anyway
            debug!("Player {} inventory full for category {:?}", player_id, category);
            let mut writer = MessageWriter::new();
            writer
                .write_u16(MessageType::ShopBuyFail.id())
                .write_u8(2); // Use case 2 for inventory full
            return Ok(vec![writer.into_bytes()]);
        }
    };

    // All checks passed - process the purchase

    // 1. Add item to player's inventory
    match category {
        ShopCategory::Outfit => {
            crate::db::update_outfit_slot(&server.db, character_id, slot, shop_item.item_id as i16)
                .await?;
        }
        ShopCategory::Item => {
            crate::db::update_item_slot(&server.db, character_id, slot, shop_item.item_id as i16)
                .await?;
        }
        ShopCategory::Accessory => {
            crate::db::update_accessory_slot(
                &server.db,
                character_id,
                slot,
                shop_item.item_id as i16,
            )
            .await?;
        }
        ShopCategory::Tool => {
            crate::db::update_tool_slot(&server.db, character_id, slot, shop_item.item_id as i16)
                .await?;
        }
    }

    // 2. Deduct points
    let new_points = points - shop_item.price as u32;
    crate::db::update_points(&server.db, character_id, new_points as i64).await?;

    // 3. Update session points
    {
        let mut session_guard = session.write().await;
        session_guard.points = new_points;
    }

    // 4. If limited item, decrement stock
    if shop_item.is_limited != 0 && shop_item.stock > 0 {
        let new_stock = shop_item.stock - 1;
        update_shop_stock(&server.db, room_id, pos_id, new_stock as u8).await?;

        // If now sold out, broadcast to all players in room
        if new_stock == 0 {
            let room_players = server.game_state.get_room_players(room_id).await;
            for other_player_id in room_players {
                if other_player_id == player_id {
                    continue; // Skip buyer, they get ShopBuy response
                }
                if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id)
                {
                    if let Some(other_session) = server.sessions.get(&other_session_id) {
                        let mut writer = MessageWriter::new();
                        writer
                            .write_u16(MessageType::ShopStock.id())
                            .write_u8(1) // case 1 = sold out
                            .write_u8(pos_id);
                        other_session
                            .write()
                            .await
                            .queue_message(writer.into_bytes());
                    }
                }
            }
        }
    }

    info!(
        "Player {} bought {:?} {} for {} points (slot {})",
        player_id, category, shop_item.item_id, shop_item.price, slot
    );

    // Send success response
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::ShopBuy.id())
        .write_u8(shop_item.category as u8)
        .write_u8(slot)
        .write_u16(shop_item.item_id as u16)
        .write_u16(shop_item.price as u16);

    Ok(vec![writer.into_bytes()])
}

/// Send MSG_ROOM_SHOP_INFO (27) to a player entering a room with shops
pub async fn send_room_shop_info(
    server: &Arc<Server>,
    session: &Arc<RwLock<PlayerSession>>,
    room_id: u16,
) -> Result<()> {
    // Get shop items for this room
    let shop_items = get_room_shop_items(&server.db, room_id).await?;

    if shop_items.is_empty() {
        return Ok(()); // No shop in this room
    }

    let count = shop_items.len() as u8;

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::RoomShopInfo.id())
        .write_u8(count);

    if count == 1 {
        // Special case: single item includes slot_id first
        let item = &shop_items[0];
        writer
            .write_u8(item.slot_id as u8)
            .write_u8(item.category as u8)
            .write_u16(item.price as u16)
            .write_u8(item.stock as u8)
            .write_u16(item.item_id as u16);
    } else {
        // Multiple items: slot is implicit (1, 2, 3, ...)
        for item in &shop_items {
            writer
                .write_u8(item.category as u8)
                .write_u16(item.price as u16)
                .write_u8(item.stock as u8)
                .write_u16(item.item_id as u16);
        }
    }

    // Queue the message for the player
    session.write().await.queue_message(writer.into_bytes());

    debug!("Sent shop info for room {} ({} items)", room_id, count);

    Ok(())
}
