//! Storage extension system handlers
//!
//! Handles extended storage messages:
//! - MSG_STORAGE_REQ (56) - Request storage page contents
//! - MSG_STORAGE_PAGES (57) - Request page fill status
//! - MSG_STORAGE_MOVE (58) - Move/swap items between storage and inventory

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::db;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

/// Storage categories
const CAT_OUTFITS: u8 = 1;
const CAT_ITEMS: u8 = 2;
const CAT_ACCESSORIES: u8 = 3;
const CAT_TOOLS: u8 = 4;

/// Slots per page in storage
const SLOTS_PER_PAGE: usize = 9;

/// Total pages in storage
const TOTAL_PAGES: usize = 20;

/// Handle MSG_STORAGE_REQ (56) - Request storage page contents
pub async fn handle_storage_req(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let category = reader.read_u8()?;
    let page = reader.read_u8()?;

    let char_id = {
        let session_guard = session.read().await;
        session_guard.character_id
    };

    let char_id = match char_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    debug!(
        "Storage request: category={}, page={}, char_id={}",
        category, page, char_id
    );

    // Validate page (1-20)
    if page < 1 || page > TOTAL_PAGES as u8 {
        warn!("Invalid storage page: {}", page);
        return Ok(vec![]);
    }

    // Get storage contents for this category and page
    let storage = db::get_storage(&server.db, char_id, category).await?;

    // Calculate start index for this page (1-based page)
    let start_idx = ((page - 1) as usize) * SLOTS_PER_PAGE;

    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::StorageReq.id());

    // Write 9 item IDs for this page
    for i in 0..SLOTS_PER_PAGE {
        let idx = start_idx + i;
        let item_id = if idx < storage.len() {
            storage[idx]
        } else {
            0
        };
        writer.write_u16(item_id);
    }

    Ok(vec![writer.into_bytes()])
}

/// Handle MSG_STORAGE_PAGES (57) - Request page fill status
pub async fn handle_storage_pages(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let category = reader.read_u8()?;

    let char_id = {
        let session_guard = session.read().await;
        session_guard.character_id
    };

    let char_id = match char_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    debug!(
        "Storage pages request: category={}, char_id={}",
        category, char_id
    );

    // Get storage contents for this category
    let storage = db::get_storage(&server.db, char_id, category).await?;

    let mut responses = Vec::new();

    // Send page fill status
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::StoragePages.id());
    writer.write_u8(0); // Response type 0 = page fill info

    // Write 20 bytes indicating if each page has items
    for page in 0..TOTAL_PAGES {
        let start_idx = page * SLOTS_PER_PAGE;
        let mut has_items = false;

        for i in 0..SLOTS_PER_PAGE {
            let idx = start_idx + i;
            if idx < storage.len() && storage[idx] != 0 {
                has_items = true;
                break;
            }
        }

        writer.write_u8(if has_items { 1 } else { 0 });
    }
    responses.push(writer.into_bytes());

    // Also send first page contents (MSG_STORAGE_REQ response)
    let mut first_page = MessageWriter::new();
    first_page.write_u16(MessageType::StorageReq.id());
    for i in 0..SLOTS_PER_PAGE {
        let item_id = if i < storage.len() { storage[i] } else { 0 };
        first_page.write_u16(item_id);
    }
    responses.push(first_page.into_bytes());

    Ok(responses)
}

/// Handle MSG_STORAGE_MOVE (58) - Move/swap items between storage and inventory
pub async fn handle_storage_move(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let category = reader.read_u8()?;
    let page = reader.read_u8()?;
    let slot_first = reader.read_u8()?;
    let slot_second = reader.read_u8()?;

    let char_id = {
        let session_guard = session.read().await;
        session_guard.character_id
    };

    let char_id = match char_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    debug!(
        "Storage move: category={}, page={}, first={}, second={}, char_id={}",
        category, page, slot_first, slot_second, char_id
    );

    // Slots 1-9 = storage on current page
    // Slots 10-18 = inventory slots
    let first_is_storage = slot_first <= 9;
    let second_is_storage = slot_second <= 9;

    // Load current inventory
    let inventory = match db::get_inventory(&server.db, char_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };

    // Load storage
    let mut storage = db::get_storage(&server.db, char_id, category).await?;

    // Ensure storage has enough slots
    let required_slots = TOTAL_PAGES * SLOTS_PER_PAGE;
    while storage.len() < required_slots {
        storage.push(0);
    }

    // Get item values based on category
    let mut inv_items: [u16; 9] = match category {
        CAT_OUTFITS => inventory.outfits(),
        CAT_ITEMS => inventory.items(),
        CAT_ACCESSORIES => inventory.accessories(),
        CAT_TOOLS => {
            let tools = inventory.tools();
            [
                tools[0] as u16,
                tools[1] as u16,
                tools[2] as u16,
                tools[3] as u16,
                tools[4] as u16,
                tools[5] as u16,
                tools[6] as u16,
                tools[7] as u16,
                tools[8] as u16,
            ]
        }
        _ => return Ok(vec![]),
    };

    // Calculate storage indices
    let storage_start = ((page - 1) as usize) * SLOTS_PER_PAGE;

    // Get first item value
    let first_val = if first_is_storage {
        let idx = storage_start + (slot_first - 1) as usize;
        storage.get(idx).copied().unwrap_or(0)
    } else {
        let idx = (slot_first - 10) as usize;
        inv_items.get(idx).copied().unwrap_or(0)
    };

    // Get second item value
    let second_val = if second_is_storage {
        let idx = storage_start + (slot_second - 1) as usize;
        storage.get(idx).copied().unwrap_or(0)
    } else {
        let idx = (slot_second - 10) as usize;
        inv_items.get(idx).copied().unwrap_or(0)
    };

    // If both empty, nothing to do
    if first_val == 0 && second_val == 0 {
        return Ok(vec![]);
    }

    // Swap the items
    if first_is_storage {
        let idx = storage_start + (slot_first - 1) as usize;
        if idx < storage.len() {
            storage[idx] = second_val;
        }
    } else {
        let idx = (slot_first - 10) as usize;
        if idx < inv_items.len() {
            inv_items[idx] = second_val;
        }
    }

    if second_is_storage {
        let idx = storage_start + (slot_second - 1) as usize;
        if idx < storage.len() {
            storage[idx] = first_val;
        }
    } else {
        let idx = (slot_second - 10) as usize;
        if idx < inv_items.len() {
            inv_items[idx] = first_val;
        }
    }

    // Save storage
    let _ = db::save_storage(&server.db, char_id, category, &storage).await;

    // Save inventory based on category
    match category {
        CAT_OUTFITS => {
            let _ = db::update_inventory_outfits(&server.db, char_id, &inv_items).await;
        }
        CAT_ITEMS => {
            let _ = db::update_inventory_items(&server.db, char_id, &inv_items).await;
        }
        CAT_ACCESSORIES => {
            let _ = db::update_inventory_accessories(&server.db, char_id, &inv_items).await;
        }
        CAT_TOOLS => {
            // Unequip tool if moving tool category
            let _ = db::update_equipped_tool(&server.db, char_id, 0).await;
            // Tools only have 3 slots
            let tools: [u16; 3] = [
                inv_items[0],
                inv_items[1],
                inv_items[2],
            ];
            let _ = db::update_inventory_tools(&server.db, char_id, &tools).await;
        }
        _ => {}
    }

    // Determine page status
    let page_status = if first_is_storage == second_is_storage {
        // Both on same side (storage-storage or inv-inv), no page change
        3
    } else {
        // Check if current storage page still has items
        let mut has_items = false;
        for i in 0..SLOTS_PER_PAGE {
            let idx = storage_start + i;
            if idx < storage.len() && storage[idx] != 0 {
                has_items = true;
                break;
            }
        }
        if has_items {
            1
        } else {
            0
        }
    };

    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::StorageMove.id());
    writer.write_u8(page_status);
    writer.write_u8(slot_second);

    Ok(vec![writer.into_bytes()])
}
