//! Item system handlers for Slime Online 2
//!
//! Handles:
//! - MSG_USE_ITEM (31) - Use item from inventory
//! - MSG_DISCARD_ITEM (39) - Drop item on ground
//! - MSG_DISCARDED_ITEM_TAKE (40) - Pick up dropped item
//! - MSG_GET_ITEM (41) - Server -> Client item received
//!
//! Item data sourced from client's db_items.gml

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

// =============================================================================
// ITEM DEFINITIONS (from db_items.gml)
// =============================================================================

/// Item categories for inventory management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ItemCategory {
    Outfit = 1,
    Item = 2,
    Accessory = 3,
    Tool = 4,
}

impl ItemCategory {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Outfit),
            2 => Some(Self::Item),
            3 => Some(Self::Accessory),
            4 => Some(Self::Tool),
            _ => None,
        }
    }
}

/// Item type for special handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    WarpWing,       // 1 - Teleport to save point
    Smokebomb,      // 2 - Visual effect
    Applebomb,      // 3 - Visual effect
    Bubbles,        // 4 - Visual effect
    Slimebag50,     // 5 - Gives 50 points
    Slimebag200,    // 6 - Gives 200 points
    Slimebag500,    // 7 - Gives 500 points
    ChickenMine,    // 8 - Place a mine
    SimpleSeed,     // 9 - Plant a tree
    Fairy,          // 10 - Use on tree (+10% fruit chance)
    BluePinwheel,   // 11 - Use on tree (-20% grow time)
    RedPinwheel,    // 12 - Use on tree (-35% grow time)
    GlowPinwheel,   // 13 - Use on tree (-50% grow time)
    Soundmaker,     // 14-19 - Play sound
    Material,       // 20-25, 39-50, 57-61 - Collectible/crafting materials
    WeakCannonKit,  // 26 - Build a weak cannon
    Gum,            // 27-32 - Place gum on ground
    LuckyCoin,      // 33 - Gambling item
    Soda,           // 34-36 - Decorative sodas
    SpeedSoda,      // 37 - Temporary speed boost
    JumpSoda,       // 38 - Temporary jump boost
    BlueSeed,       // 24 - Plant a special tree
    ProofStone,     // 51-56 - Proof of conquest items
    Generic,        // Other items
}

/// Item information
#[derive(Debug, Clone)]
pub struct ItemInfo {
    pub id: u16,
    pub name: &'static str,
    pub item_type: ItemType,
    pub is_consumable: bool,
    pub is_discardable: bool,
}

/// Get item information by ID (from db_items.gml)
pub fn get_item_info(item_id: u16) -> Option<ItemInfo> {
    match item_id {
        1 => Some(ItemInfo {
            id: 1,
            name: "Warp-Wing",
            item_type: ItemType::WarpWing,
            is_consumable: true,
            is_discardable: true,
        }),
        2 => Some(ItemInfo {
            id: 2,
            name: "Smokebomb",
            item_type: ItemType::Smokebomb,
            is_consumable: true,
            is_discardable: true,
        }),
        3 => Some(ItemInfo {
            id: 3,
            name: "Applebomb",
            item_type: ItemType::Applebomb,
            is_consumable: true,
            is_discardable: true,
        }),
        4 => Some(ItemInfo {
            id: 4,
            name: "Bubbles",
            item_type: ItemType::Bubbles,
            is_consumable: true,
            is_discardable: true,
        }),
        5 => Some(ItemInfo {
            id: 5,
            name: "Slimebag [50]",
            item_type: ItemType::Slimebag50,
            is_consumable: true,
            is_discardable: true,
        }),
        6 => Some(ItemInfo {
            id: 6,
            name: "Slimebag [200]",
            item_type: ItemType::Slimebag200,
            is_consumable: true,
            is_discardable: true,
        }),
        7 => Some(ItemInfo {
            id: 7,
            name: "Slimebag [500]",
            item_type: ItemType::Slimebag500,
            is_consumable: true,
            is_discardable: true,
        }),
        8 => Some(ItemInfo {
            id: 8,
            name: "Chicken Mine",
            item_type: ItemType::ChickenMine,
            is_consumable: true,
            is_discardable: true,
        }),
        9 => Some(ItemInfo {
            id: 9,
            name: "Simple Seed",
            item_type: ItemType::SimpleSeed,
            is_consumable: true,
            is_discardable: true,
        }),
        10 => Some(ItemInfo {
            id: 10,
            name: "Fairy",
            item_type: ItemType::Fairy,
            is_consumable: true,
            is_discardable: true,
        }),
        11 => Some(ItemInfo {
            id: 11,
            name: "Blue Pinwheel",
            item_type: ItemType::BluePinwheel,
            is_consumable: true,
            is_discardable: true,
        }),
        12 => Some(ItemInfo {
            id: 12,
            name: "Red Pinwheel",
            item_type: ItemType::RedPinwheel,
            is_consumable: true,
            is_discardable: true,
        }),
        13 => Some(ItemInfo {
            id: 13,
            name: "Glow Pinwheel",
            item_type: ItemType::GlowPinwheel,
            is_consumable: true,
            is_discardable: true,
        }),
        14 => Some(ItemInfo {
            id: 14,
            name: "Rockman Sound",
            item_type: ItemType::Soundmaker,
            is_consumable: true,
            is_discardable: true,
        }),
        15 => Some(ItemInfo {
            id: 15,
            name: "Kirby Sound",
            item_type: ItemType::Soundmaker,
            is_consumable: true,
            is_discardable: true,
        }),
        16 => Some(ItemInfo {
            id: 16,
            name: "Link Sound",
            item_type: ItemType::Soundmaker,
            is_consumable: true,
            is_discardable: true,
        }),
        17 => Some(ItemInfo {
            id: 17,
            name: "Pipe Sound",
            item_type: ItemType::Soundmaker,
            is_consumable: true,
            is_discardable: true,
        }),
        18 => Some(ItemInfo {
            id: 18,
            name: "DK Sound",
            item_type: ItemType::Soundmaker,
            is_consumable: true,
            is_discardable: true,
        }),
        19 => Some(ItemInfo {
            id: 19,
            name: "Metroid Sound",
            item_type: ItemType::Soundmaker,
            is_consumable: true,
            is_discardable: true,
        }),
        20 => Some(ItemInfo {
            id: 20,
            name: "Red Mushroom",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        21 => Some(ItemInfo {
            id: 21,
            name: "Tailphire",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        22 => Some(ItemInfo {
            id: 22,
            name: "Magmanis",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        23 => Some(ItemInfo {
            id: 23,
            name: "Bright Drink",
            item_type: ItemType::Generic,
            is_consumable: true,
            is_discardable: true,
        }),
        24 => Some(ItemInfo {
            id: 24,
            name: "Blue Seed",
            item_type: ItemType::BlueSeed,
            is_consumable: true,
            is_discardable: true,
        }),
        25 => Some(ItemInfo {
            id: 25,
            name: "Juicy Bango",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        26 => Some(ItemInfo {
            id: 26,
            name: "Weak Cannon Kit",
            item_type: ItemType::WeakCannonKit,
            is_consumable: true,
            is_discardable: true,
        }),
        27 => Some(ItemInfo {
            id: 27,
            name: "Red Gum",
            item_type: ItemType::Gum,
            is_consumable: true,
            is_discardable: true,
        }),
        28 => Some(ItemInfo {
            id: 28,
            name: "Orange Gum",
            item_type: ItemType::Gum,
            is_consumable: true,
            is_discardable: true,
        }),
        29 => Some(ItemInfo {
            id: 29,
            name: "Green Gum",
            item_type: ItemType::Gum,
            is_consumable: true,
            is_discardable: true,
        }),
        30 => Some(ItemInfo {
            id: 30,
            name: "Blue Gum",
            item_type: ItemType::Gum,
            is_consumable: true,
            is_discardable: true,
        }),
        31 => Some(ItemInfo {
            id: 31,
            name: "Pink Gum",
            item_type: ItemType::Gum,
            is_consumable: true,
            is_discardable: true,
        }),
        32 => Some(ItemInfo {
            id: 32,
            name: "White Gum",
            item_type: ItemType::Gum,
            is_consumable: true,
            is_discardable: true,
        }),
        33 => Some(ItemInfo {
            id: 33,
            name: "Lucky Coin",
            item_type: ItemType::LuckyCoin,
            is_consumable: false,
            is_discardable: true,
        }),
        34 => Some(ItemInfo {
            id: 34,
            name: "Bunny Soda",
            item_type: ItemType::Soda,
            is_consumable: true,
            is_discardable: true,
        }),
        35 => Some(ItemInfo {
            id: 35,
            name: "Slime Soda",
            item_type: ItemType::Soda,
            is_consumable: true,
            is_discardable: true,
        }),
        36 => Some(ItemInfo {
            id: 36,
            name: "Penguin Soda",
            item_type: ItemType::Soda,
            is_consumable: true,
            is_discardable: true,
        }),
        37 => Some(ItemInfo {
            id: 37,
            name: "Speed Soda",
            item_type: ItemType::SpeedSoda,
            is_consumable: true,
            is_discardable: true,
        }),
        38 => Some(ItemInfo {
            id: 38,
            name: "Jump Soda",
            item_type: ItemType::JumpSoda,
            is_consumable: true,
            is_discardable: true,
        }),
        39 => Some(ItemInfo {
            id: 39,
            name: "Sleenmium",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        40 => Some(ItemInfo {
            id: 40,
            name: "Sledmium",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        41 => Some(ItemInfo {
            id: 41,
            name: "Sluemium",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        42 => Some(ItemInfo {
            id: 42,
            name: "Slinkmium",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        43 => Some(ItemInfo {
            id: 43,
            name: "Slelloymium",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        44 => Some(ItemInfo {
            id: 44,
            name: "Slaymium",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        45 => Some(ItemInfo {
            id: 45,
            name: "Slackmium",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        46 => Some(ItemInfo {
            id: 46,
            name: "Screw",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        47 => Some(ItemInfo {
            id: 47,
            name: "Rusty Screw",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        48 => Some(ItemInfo {
            id: 48,
            name: "Bug Leg",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: false, // Not in discard list
        }),
        49 => Some(ItemInfo {
            id: 49,
            name: "Weird Coin",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        50 => Some(ItemInfo {
            id: 50,
            name: "Firestone",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        51 => Some(ItemInfo {
            id: 51,
            name: "Proof of Nature",
            item_type: ItemType::ProofStone,
            is_consumable: false,
            is_discardable: true,
        }),
        52 => Some(ItemInfo {
            id: 52,
            name: "Proof of Earth",
            item_type: ItemType::ProofStone,
            is_consumable: false,
            is_discardable: true,
        }),
        53 => Some(ItemInfo {
            id: 53,
            name: "Proof of Water",
            item_type: ItemType::ProofStone,
            is_consumable: false,
            is_discardable: true,
        }),
        54 => Some(ItemInfo {
            id: 54,
            name: "Proof of Fire",
            item_type: ItemType::ProofStone,
            is_consumable: false,
            is_discardable: true,
        }),
        55 => Some(ItemInfo {
            id: 55,
            name: "Proof of Stone",
            item_type: ItemType::ProofStone,
            is_consumable: false,
            is_discardable: true,
        }),
        56 => Some(ItemInfo {
            id: 56,
            name: "Proof of Wind",
            item_type: ItemType::ProofStone,
            is_consumable: false,
            is_discardable: true,
        }),
        57 => Some(ItemInfo {
            id: 57,
            name: "Blazing Bubble",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        58 => Some(ItemInfo {
            id: 58,
            name: "Squishy Mushroom",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        59 => Some(ItemInfo {
            id: 59,
            name: "Stinky Mushroom",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        60 => Some(ItemInfo {
            id: 60,
            name: "Bell Twig",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        61 => Some(ItemInfo {
            id: 61,
            name: "Irrlicht",
            item_type: ItemType::Material,
            is_consumable: false,
            is_discardable: true,
        }),
        _ => None,
    }
}

/// Check if an item can be discarded (from item_discard_slot.gml)
pub fn can_discard_item(item_id: u16) -> bool {
    matches!(
        item_id,
        1..=47 | 49..=61
    )
}

// =============================================================================
// MSG_USE_ITEM (31) - Use item from inventory
// =============================================================================

/// Handle MSG_USE_ITEM (31)
/// Client sends format varies by item type (see item_use_slot.gml):
/// - Most items: slot (1 byte) + x (2 bytes) + y (2 bytes)
/// - Slimebags/Chicken Mine/Bright Drink/Sodas: slot (1 byte) only
/// - Bubbles: slot (1 byte) + x (2 bytes) + y (2 bytes) + direction (1 byte)
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

    // Validate slot (1-9)
    if slot < 1 || slot > 9 {
        warn!("Invalid item slot: {}", slot);
        return Ok(vec![]);
    }

    let (character_id, player_id, room_id, session_x, session_y) = {
        let session_guard = session.read().await;
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }
        (
            session_guard.character_id,
            session_guard.player_id,
            session_guard.room_id,
            session_guard.x,
            session_guard.y,
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

    let x = use_x.unwrap_or(session_x);
    let y = use_y.unwrap_or(session_y);

    // Handle item effects based on type
    match item_info.item_type {
        ItemType::WarpWing => {
            // Warp-Wing: Teleport to spawn/save point
            // Server responds with: item_id (2) + self (1) + room (2) + x (2) + y (2)
            let spawn_x = crate::constants::DEFAULT_SPAWN_X;
            let spawn_y = crate::constants::DEFAULT_SPAWN_Y;
            let spawn_room = crate::constants::DEFAULT_SPAWN_ROOM;

            // Send use item response for warp effect
            let mut writer = MessageWriter::new();
            writer
                .write_u16(MessageType::UseItem.id())
                .write_u16(item_id)
                .write_u8(1) // self = true
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

            // Update database position
            crate::db::update_position(
                &server.db,
                character_id,
                spawn_x as i16,
                spawn_y as i16,
                spawn_room as i16,
            )
            .await?;

            // Remove item from inventory (consumed on client side already)
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::Smokebomb | ItemType::Applebomb => {
            // Broadcast effect to room
            let room_players = server.game_state.get_room_players(room_id).await;
            for other_player_id in room_players {
                if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id)
                {
                    if let Some(other_session) = server.sessions.get(&other_session_id) {
                        let mut writer = MessageWriter::new();
                        writer
                            .write_u16(MessageType::UseItem.id())
                            .write_u16(item_id)
                            .write_u16(x)
                            .write_u16(y);
                        other_session
                            .write()
                            .await
                            .queue_message(writer.into_bytes());
                    }
                }
            }
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::Bubbles => {
            // Bubbles: extra byte for direction
            let direction = reader.read_u8().unwrap_or(0);
            let amount = 5u8; // Default bubble amount

            let room_players = server.game_state.get_room_players(room_id).await;
            for other_player_id in room_players {
                if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id)
                {
                    if let Some(other_session) = server.sessions.get(&other_session_id) {
                        let mut writer = MessageWriter::new();
                        writer
                            .write_u16(MessageType::UseItem.id())
                            .write_u16(item_id)
                            .write_u16(x)
                            .write_u16(y)
                            .write_u8(direction)
                            .write_u8(amount);
                        other_session
                            .write()
                            .await
                            .queue_message(writer.into_bytes());
                    }
                }
            }
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::Slimebag50 | ItemType::Slimebag200 | ItemType::Slimebag500 => {
            // Give points to player
            let points_to_add: i64 = match item_info.item_type {
                ItemType::Slimebag50 => 50,
                ItemType::Slimebag200 => 200,
                ItemType::Slimebag500 => 500,
                _ => 0,
            };

            // Get current points and add
            let character =
                crate::db::find_character_by_account(&server.db, session.read().await.account_id.unwrap_or(0))
                    .await?;
            if let Some(char) = character {
                let new_points = (char.points + points_to_add).min(crate::constants::MAX_POINTS as i64);
                crate::db::update_points(&server.db, character_id, new_points).await?;

                // Update session
                session.write().await.points = new_points as u32;
            }

            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::ChickenMine => {
            // Place chicken mine at player position
            // Broadcast to room that mine was placed
            let room_players = server.game_state.get_room_players(room_id).await;
            for other_player_id in room_players {
                if other_player_id == player_id {
                    // Send confirmation to placer
                    let mut writer = MessageWriter::new();
                    writer.write_u16(MessageType::UseItem.id()).write_u16(item_id);
                    responses.push(writer.into_bytes());
                }
                // Other players see the mine placed (handled by client effect)
            }
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::Soundmaker => {
            // Broadcast sound to room
            let room_players = server.game_state.get_room_players(room_id).await;
            for other_player_id in room_players {
                if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id)
                {
                    if let Some(other_session) = server.sessions.get(&other_session_id) {
                        let mut writer = MessageWriter::new();
                        writer
                            .write_u16(MessageType::UseItem.id())
                            .write_u16(item_id)
                            .write_u16(player_id);
                        other_session
                            .write()
                            .await
                            .queue_message(writer.into_bytes());
                    }
                }
            }
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::Gum => {
            // Place gum on ground - broadcast to room
            let room_players = server.game_state.get_room_players(room_id).await;
            for other_player_id in room_players {
                if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id)
                {
                    if let Some(other_session) = server.sessions.get(&other_session_id) {
                        let mut writer = MessageWriter::new();
                        writer
                            .write_u16(MessageType::UseItem.id())
                            .write_u16(item_id)
                            .write_u16(x)
                            .write_u16(y);
                        other_session
                            .write()
                            .await
                            .queue_message(writer.into_bytes());
                    }
                }
            }
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::Soda | ItemType::SpeedSoda | ItemType::JumpSoda => {
            // Sodas - consume and apply effect (client handles visual)
            // Speed/Jump sodas give temporary buffs (client-side timing)
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::SimpleSeed | ItemType::BlueSeed => {
            // Seeds require special planting logic - handled by scr_plant_seed
            // For now, just consume
            debug!("Seed planting not fully implemented yet");
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        ItemType::Fairy | ItemType::BluePinwheel | ItemType::RedPinwheel | ItemType::GlowPinwheel => {
            // These require targeting a tree - handled by separate scripts
            debug!("Tree enhancement items not fully implemented yet");
        }

        ItemType::WeakCannonKit => {
            // Build cannon - requires build spot
            debug!("Cannon building not fully implemented yet");
            crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;
        }

        _ => {
            // Generic/material items - cannot be used
            debug!("Item {} cannot be used", item_id);
        }
    }

    Ok(responses)
}

// =============================================================================
// MSG_DISCARD_ITEM (39) - Drop item on ground
// =============================================================================

/// Handle MSG_DISCARD_ITEM (39)
/// Client sends: slot (1 byte) + x (2 bytes) + y (2 bytes)
/// Server broadcasts: x (2) + y (2) + item_id (2) + instance_id (2)
pub async fn handle_discard_item(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.len() < 5 {
        return Ok(vec![]);
    }

    let mut reader = MessageReader::new(payload);
    let slot = reader.read_u8()?;
    let drop_x = reader.read_u16()?;
    let drop_y = reader.read_u16()?;

    // Validate slot (1-9)
    if slot < 1 || slot > 9 {
        warn!("Invalid discard slot: {}", slot);
        return Ok(vec![]);
    }

    let (character_id, player_id, room_id) = {
        let session_guard = session.read().await;
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }
        (
            session_guard.character_id,
            session_guard.player_id,
            session_guard.room_id,
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

    // Get the item in this slot from database
    let inventory = match crate::db::get_inventory(&server.db, character_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };

    let items = inventory.items();
    let item_id = items[(slot - 1) as usize];

    if item_id == 0 {
        debug!("Slot {} is empty, nothing to discard", slot);
        return Ok(vec![]);
    }

    // Check if item can be discarded
    if !can_discard_item(item_id) {
        debug!("Item {} cannot be discarded", item_id);
        return Ok(vec![]);
    }

    info!(
        "Player {} discarding item {} from slot {} at ({}, {}) in room {}",
        player_id, item_id, slot, drop_x, drop_y, room_id
    );

    // Remove item from inventory
    crate::db::update_item_slot(&server.db, character_id, slot, 0).await?;

    // Add to dropped items in game state and get instance ID
    let instance_id = server
        .game_state
        .add_dropped_item(room_id, drop_x, drop_y, item_id)
        .await;

    // Broadcast to all players in the room
    let room_players = server.game_state.get_room_players(room_id).await;
    for other_player_id in room_players {
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                writer
                    .write_u16(MessageType::DiscardItem.id())
                    .write_u16(drop_x)
                    .write_u16(drop_y)
                    .write_u16(item_id)
                    .write_u16(instance_id);
                other_session
                    .write()
                    .await
                    .queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

// =============================================================================
// MSG_DISCARDED_ITEM_TAKE (40) - Pick up dropped item
// =============================================================================

/// Handle MSG_DISCARDED_ITEM_TAKE (40)
/// Client sends: instance_id (2 bytes)
/// Server responds with MSG_GET_ITEM if successful
pub async fn handle_take_dropped_item(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.len() < 2 {
        return Ok(vec![]);
    }

    let mut reader = MessageReader::new(payload);
    let instance_id = reader.read_u16()?;

    let (character_id, player_id, room_id) = {
        let session_guard = session.read().await;
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }
        (
            session_guard.character_id,
            session_guard.player_id,
            session_guard.room_id,
        )
    };

    let character_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Try to take the dropped item
    let dropped_item = match server
        .game_state
        .take_dropped_item(room_id, instance_id)
        .await
    {
        Some(item) => item,
        None => {
            debug!("Dropped item {} not found or already taken", instance_id);
            return Ok(vec![]);
        }
    };

    // Find an empty slot in inventory
    let inventory = match crate::db::get_inventory(&server.db, character_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };

    let items = inventory.items();
    let empty_slot = items.iter().position(|&id| id == 0);

    let slot = match empty_slot {
        Some(idx) => (idx + 1) as u8, // Slots are 1-indexed
        None => {
            // Inventory full - put the item back
            server
                .game_state
                .add_dropped_item_with_id(
                    room_id,
                    dropped_item.x,
                    dropped_item.y,
                    dropped_item.item_id,
                    instance_id,
                )
                .await;
            debug!("Inventory full, cannot pick up item");
            return Ok(vec![]);
        }
    };

    info!(
        "Player {:?} picking up item {} (instance {}) into slot {}",
        player_id, dropped_item.item_id, instance_id, slot
    );

    // Add item to inventory
    crate::db::update_item_slot(&server.db, character_id, slot, dropped_item.item_id as i16)
        .await?;

    // Send MSG_GET_ITEM to player
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::GetItem.id())
        .write_u8(slot)
        .write_u16(dropped_item.item_id);

    Ok(vec![writer.into_bytes()])
}

// =============================================================================
// HELPER: Write MSG_GET_ITEM response
// =============================================================================

/// Create a MSG_GET_ITEM message
pub fn write_get_item(slot: u8, item_id: u16) -> Vec<u8> {
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::GetItem.id())
        .write_u8(slot)
        .write_u16(item_id);
    writer.into_bytes()
}
