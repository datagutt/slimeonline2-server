//! Shop sell handler for Slime Online 2
//!
//! MSG_SELL_REQ_PRICES (53):
//!   Client -> Server: category (u8) - 1=outfits, 2=items, 3=accessories, 4=tools
//!   Server -> Client: prices for each FILLED slot (u16 each, only for non-empty slots)
//!
//! MSG_SELL (54):
//!   Client -> Server: category (u8) + count (u8) + slots[count] (u8 each)
//!   Server -> Client: total_points_earned (u32)

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;
use crate::handlers::items::get_sell_price;

/// Get sell price for an outfit by ID (server decides prices)
fn get_outfit_sell_price(outfit_id: u16) -> u16 {
    // Outfits generally sell for 50-200 points depending on rarity
    // For now, use a simple formula based on ID ranges
    match outfit_id {
        0 => 0,
        1..=10 => 50,      // Common outfits
        11..=30 => 100,    // Uncommon
        31..=50 => 150,    // Rare
        _ => 200,          // Very rare
    }
}

/// Get sell price for an accessory by ID
fn get_accessory_sell_price(accessory_id: u16) -> u16 {
    match accessory_id {
        0 => 0,
        1..=10 => 30,      // Common accessories
        11..=30 => 75,     // Uncommon
        31..=50 => 125,    // Rare
        _ => 175,          // Very rare
    }
}

/// Get sell price for a tool by ID
fn get_tool_sell_price(tool_id: u16) -> u16 {
    match tool_id {
        0 => 0,
        1 => 100,  // Basic tools
        2 => 150,
        3 => 200,
        4 => 250,
        5 => 300,
        _ => 100,
    }
}

/// Handle MSG_SELL_REQ_PRICES (53)
/// Client requests sell prices for a category
pub async fn handle_sell_req_prices(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }
    
    let mut reader = MessageReader::new(payload);
    let category = reader.read_u8()?;
    
    let character_id = session.read().await.character_id;
    
    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };
    
    // Get player's inventory
    let inventory = match crate::db::get_inventory(&server.db, char_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };
    
    // Build response - only send prices for filled slots
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::SellReqPrices.id());
    
    match category {
        1 => {
            // Outfits
            let outfits = inventory.outfits();
            for &outfit_id in outfits.iter() {
                if outfit_id != 0 {
                    writer.write_u16(get_outfit_sell_price(outfit_id));
                }
            }
            debug!("Sending sell prices for outfits");
        }
        2 => {
            // Items
            let items = inventory.items();
            for &item_id in items.iter() {
                if item_id != 0 {
                    writer.write_u16(get_sell_price(item_id));
                }
            }
            debug!("Sending sell prices for items");
        }
        3 => {
            // Accessories
            let accessories = inventory.accessories();
            for &acs_id in accessories.iter() {
                if acs_id != 0 {
                    writer.write_u16(get_accessory_sell_price(acs_id));
                }
            }
            debug!("Sending sell prices for accessories");
        }
        4 => {
            // Tools
            let tools = inventory.tools();
            for &tool_id in tools.iter() {
                if tool_id != 0 {
                    writer.write_u16(get_tool_sell_price(tool_id as u16));
                }
            }
            debug!("Sending sell prices for tools");
        }
        _ => {
            warn!("Invalid sell category: {}", category);
            return Ok(vec![]);
        }
    }
    
    Ok(vec![writer.into_bytes()])
}

/// Handle MSG_SELL (54)
/// Client wants to sell items from multiple slots
pub async fn handle_sell(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.len() < 2 {
        return Ok(vec![]);
    }
    
    let mut reader = MessageReader::new(payload);
    let category = reader.read_u8()?;
    let count = reader.read_u8()?;
    
    // Read all slots to sell
    let mut slots_to_sell = Vec::with_capacity(count as usize);
    for _ in 0..count {
        if let Ok(slot) = reader.read_u8() {
            if slot >= 1 && slot <= 9 {
                slots_to_sell.push(slot);
            }
        }
    }
    
    if slots_to_sell.is_empty() {
        return Ok(vec![]);
    }
    
    let (character_id, current_points) = {
        let session_guard = session.read().await;
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }
        (session_guard.character_id, session_guard.points)
    };
    
    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };
    
    // Get player's inventory
    let inventory = match crate::db::get_inventory(&server.db, char_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };
    
    let mut total_earned: u64 = 0;
    
    match category {
        1 => {
            // Sell outfits
            let outfits = inventory.outfits();
            for &slot in &slots_to_sell {
                let outfit_id = outfits[(slot - 1) as usize];
                if outfit_id != 0 {
                    let price = get_outfit_sell_price(outfit_id);
                    if price > 0 {
                        total_earned += price as u64;
                        crate::db::update_outfit_slot(&server.db, char_id, slot, 0).await?;
                        debug!("Sold outfit {} from slot {} for {} points", outfit_id, slot, price);
                    }
                }
            }
        }
        2 => {
            // Sell items
            let items = inventory.items();
            for &slot in &slots_to_sell {
                let item_id = items[(slot - 1) as usize];
                if item_id != 0 {
                    let price = get_sell_price(item_id);
                    if price > 0 {
                        total_earned += price as u64;
                        crate::db::update_item_slot(&server.db, char_id, slot, 0).await?;
                        debug!("Sold item {} from slot {} for {} points", item_id, slot, price);
                    }
                }
            }
        }
        3 => {
            // Sell accessories
            let accessories = inventory.accessories();
            for &slot in &slots_to_sell {
                let acs_id = accessories[(slot - 1) as usize];
                if acs_id != 0 {
                    let price = get_accessory_sell_price(acs_id);
                    if price > 0 {
                        total_earned += price as u64;
                        crate::db::update_accessory_slot(&server.db, char_id, slot, 0).await?;
                        debug!("Sold accessory {} from slot {} for {} points", acs_id, slot, price);
                    }
                }
            }
        }
        4 => {
            // Sell tools
            let tools = inventory.tools();
            for &slot in &slots_to_sell {
                let tool_id = tools[(slot - 1) as usize];
                if tool_id != 0 {
                    let price = get_tool_sell_price(tool_id as u16);
                    if price > 0 {
                        total_earned += price as u64;
                        crate::db::update_tool_slot(&server.db, char_id, slot, 0).await?;
                        debug!("Sold tool {} from slot {} for {} points", tool_id, slot, price);
                    }
                }
            }
        }
        _ => {
            warn!("Invalid sell category: {}", category);
            return Ok(vec![]);
        }
    }
    
    // Calculate new points (cap at max)
    let new_points = (current_points as u64 + total_earned).min(crate::constants::MAX_POINTS as u64) as u32;
    
    // Update points in database
    crate::db::update_points(&server.db, char_id, new_points as i64).await?;
    
    // Update session
    {
        let mut session_guard = session.write().await;
        session_guard.points = new_points;
    }
    
    info!("Player {} sold {} items from category {} for {} total points (new balance: {})", 
          char_id, slots_to_sell.len(), category, total_earned, new_points);
    
    // Send response with total points earned
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::Sell.id())
        .write_u32(total_earned as u32);
    
    Ok(vec![writer.into_bytes()])
}
