//! Quest system handlers
//!
//! Based on decompiled original server logic.
//! Quests are client-driven but server-validated.
//!
//! Messages:
//! - MSG_QUEST_BEGIN (83) - Start a quest
//! - MSG_QUEST_CLEAR (84) - Complete a quest  
//! - MSG_QUEST_STEP_INC (85) - Advance quest step
//! - MSG_QUEST_CANCEL (86) - Cancel current quest
//! - MSG_QUEST_NPC_REQ (87) - Check if quest is cleared
//! - MSG_QUEST_REWARD (92) - Claim quest reward

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::db;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

// =============================================================================
// MSG_QUEST_BEGIN (83)
// =============================================================================

/// Handle MSG_QUEST_BEGIN (83)
/// Client informs us they started a quest.
/// We set quest_id, quest_step=1, quest_var=0
pub async fn handle_quest_begin(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let quest_id = reader.read_u8()? as i16;

    let character_id = session.read().await.character_id;
    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Check if quest is already cleared
    if db::is_quest_cleared(&server.db, char_id, quest_id)
        .await
        .unwrap_or(false)
    {
        warn!(
            "HACK: Player {} tried to start already-cleared quest {}",
            char_id, quest_id
        );
        return Ok(vec![]);
    }

    // Set quest state: quest_id, step=1, var=0
    if let Err(e) = db::set_quest_state(&server.db, char_id, quest_id, 1, 0).await {
        warn!("Failed to set quest state: {}", e);
        return Ok(vec![]);
    }

    debug!("Player {} started quest {}", char_id, quest_id);
    Ok(vec![])
}

// =============================================================================
// MSG_QUEST_CANCEL (86)
// =============================================================================

/// Handle MSG_QUEST_CANCEL (86)
/// Client canceled their current quest.
/// Reset quest_id, quest_step, quest_var to 0
pub async fn handle_quest_cancel(
    _payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let character_id = session.read().await.character_id;
    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Get current quest state
    let (quest_id, _, _) = db::get_quest_state(&server.db, char_id)
        .await
        .unwrap_or((0, 0, 0));

    if quest_id == 0 {
        warn!(
            "HACK: Player {} tried to cancel quest but has no active quest",
            char_id
        );
        return Ok(vec![]);
    }

    // Reset quest state
    if let Err(e) = db::set_quest_state(&server.db, char_id, 0, 0, 0).await {
        warn!("Failed to reset quest state: {}", e);
        return Ok(vec![]);
    }

    debug!("Player {} canceled quest {}", char_id, quest_id);
    Ok(vec![])
}

// =============================================================================
// MSG_QUEST_CLEAR (84)
// =============================================================================

/// Handle MSG_QUEST_CLEAR (84)
/// Client informs us they finished a quest.
/// We validate, mark as cleared, and reset quest state.
pub async fn handle_quest_clear(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let quest_id = reader.read_u8()? as i16;

    let character_id = session.read().await.character_id;
    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Get current quest state
    let (current_quest_id, _, _) = db::get_quest_state(&server.db, char_id)
        .await
        .unwrap_or((0, 0, 0));

    if current_quest_id != quest_id {
        warn!(
            "HACK: Player {} tried to clear quest {} but current quest is {}",
            char_id, quest_id, current_quest_id
        );
        return Ok(vec![]);
    }

    // Mark quest as cleared
    if let Err(e) = db::mark_quest_cleared(&server.db, char_id, quest_id).await {
        warn!("Failed to mark quest cleared: {}", e);
        return Ok(vec![]);
    }

    // Reset quest state
    if let Err(e) = db::set_quest_state(&server.db, char_id, 0, 0, 0).await {
        warn!("Failed to reset quest state: {}", e);
        return Ok(vec![]);
    }

    debug!("Player {} cleared quest {}", char_id, quest_id);
    Ok(vec![])
}

// =============================================================================
// MSG_QUEST_STEP_INC (85)
// =============================================================================

/// Handle MSG_QUEST_STEP_INC (85)
/// Client advances quest step by 1.
pub async fn handle_quest_step_inc(
    _payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let character_id = session.read().await.character_id;
    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Get current quest state
    let (quest_id, quest_step, quest_var) = db::get_quest_state(&server.db, char_id)
        .await
        .unwrap_or((0, 0, 0));

    if quest_id == 0 {
        warn!(
            "HACK: Player {} tried to increment quest step but has no active quest",
            char_id
        );
        return Ok(vec![]);
    }

    // Increment step
    let new_step = quest_step + 1;
    if let Err(e) = db::set_quest_state(&server.db, char_id, quest_id, new_step, quest_var).await {
        warn!("Failed to update quest step: {}", e);
        return Ok(vec![]);
    }

    debug!(
        "Player {} quest {} step {} -> {}",
        char_id, quest_id, quest_step, new_step
    );
    Ok(vec![])
}

// =============================================================================
// MSG_QUEST_NPC_REQ (87)
// =============================================================================

/// Handle MSG_QUEST_NPC_REQ (87)
/// NPC asks server if a quest has been cleared.
/// Response: quest_id (byte), cleared (byte: 0=no, 1=yes)
pub async fn handle_quest_npc_req(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let quest_id = reader.read_u8()? as i16;

    let character_id = session.read().await.character_id;
    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    let cleared = db::is_quest_cleared(&server.db, char_id, quest_id)
        .await
        .unwrap_or(false);

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::QuestNpcReq.id())
        .write_u8(quest_id as u8)
        .write_u8(if cleared { 1 } else { 0 });

    debug!("Quest NPC req: quest {} cleared={}", quest_id, cleared);
    Ok(vec![writer.into_bytes()])
}

// =============================================================================
// MSG_QUEST_REWARD (92)
// =============================================================================

/// Handle MSG_QUEST_REWARD (92)
/// Client claims quest reward. We validate and give reward.
///
/// From original server (case_msg_quest_reward.gml):
/// - Quest 1 "Lazy Coolness": On step 2, exchange Bubbles (item 4) for Seed (item 9)
pub async fn handle_quest_reward(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let quest_id = reader.read_u8()? as i16;
    let quest_step = reader.read_u8()? as i16;

    let character_id = session.read().await.character_id;
    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Get current quest state from DB
    let (current_quest_id, current_step, _) = db::get_quest_state(&server.db, char_id)
        .await
        .unwrap_or((0, 0, 0));

    // Validate: must have active quest
    if current_quest_id == 0 || current_step == 0 {
        warn!(
            "HACK: Player {} requested quest reward but has no active quest",
            char_id
        );
        return Ok(vec![]);
    }

    // Validate: quest_id must match
    if quest_id != current_quest_id {
        warn!(
            "HACK: Player {} requested reward for quest {} but active quest is {}",
            char_id, quest_id, current_quest_id
        );
        return Ok(vec![]);
    }

    // Validate: quest_step must match
    if quest_step != current_step {
        warn!(
            "HACK: Player {} quest step mismatch: client={} server={}",
            char_id, quest_step, current_step
        );
        return Ok(vec![]);
    }

    // Process reward based on quest ID and step
    match quest_id {
        1 => handle_quest_1_reward(server, char_id, quest_step).await,
        _ => {
            debug!("No reward logic for quest {} step {}", quest_id, quest_step);
            Ok(vec![])
        }
    }
}

/// Quest 1: "Lazy Coolness"
/// Step 2: Give Bubbles (item 4), receive Seed (item 9)
async fn handle_quest_1_reward(
    server: &Arc<Server>,
    char_id: i64,
    quest_step: i16,
) -> Result<Vec<Vec<u8>>> {
    if quest_step != 2 {
        warn!(
            "HACK: Quest 1 reward not given on step 2, got step {}",
            quest_step
        );
        return Ok(vec![]);
    }

    // Get inventory to find Bubbles (item 4)
    let inventory = match db::get_inventory(&server.db, char_id).await {
        Ok(Some(inv)) => inv,
        Ok(None) => {
            warn!("No inventory for quest reward");
            return Ok(vec![]);
        }
        Err(e) => {
            warn!("Failed to get inventory for quest reward: {}", e);
            return Ok(vec![]);
        }
    };

    // Find slot with Bubbles (item 4)
    let items = inventory.items();
    let bubble_slot = items.iter().position(|&id| id == 4);

    let slot = match bubble_slot {
        Some(idx) => (idx + 1) as u8, // Convert 0-indexed to 1-indexed
        None => {
            warn!(
                "HACK: Player {} doesn't have Bubbles for quest 1 reward",
                char_id
            );
            return Ok(vec![]);
        }
    };

    // Replace Bubbles with Seed (item 9)
    if let Err(e) = db::update_item_slot(&server.db, char_id, slot, 9).await {
        warn!("Failed to give quest reward item: {}", e);
        return Ok(vec![]);
    }

    // Send MSG_GET_SOMETHING to update client inventory
    // Format: category (byte), slot (byte), new_item_id (u16)
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::GetSomething.id())
        .write_u8(2) // Category 2 = Items
        .write_u8(slot) // Slot number
        .write_u16(9); // New item: Seed

    debug!(
        "Quest 1 reward: gave Seed to player {} in slot {}",
        char_id, slot
    );
    Ok(vec![writer.into_bytes()])
}

// =============================================================================
// Quest Var Messages (88-90) - Used for complex quests
// =============================================================================

/// Handle MSG_QUEST_VAR_CHECK (88)
/// Not used in original server but defined in protocol
pub async fn handle_quest_var_check(
    _payload: &[u8],
    _server: &Arc<Server>,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    // Not implemented in original server
    debug!("Quest var check - not implemented");
    Ok(vec![])
}

/// Handle MSG_QUEST_VAR_INC (89)
/// Not used in original server but defined in protocol
pub async fn handle_quest_var_inc(
    _payload: &[u8],
    _server: &Arc<Server>,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    // Not implemented in original server
    debug!("Quest var inc - not implemented");
    Ok(vec![])
}

/// Handle MSG_QUEST_VAR_SET (90)
/// Not used in original server but defined in protocol
pub async fn handle_quest_var_set(
    _payload: &[u8],
    _server: &Arc<Server>,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    // Not implemented in original server
    debug!("Quest var set - not implemented");
    Ok(vec![])
}

/// Handle MSG_QUEST_STATUS_REQ (91)
/// Not used in original server but defined in protocol
pub async fn handle_quest_status_req(
    _payload: &[u8],
    _server: &Arc<Server>,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    // Not implemented in original server
    debug!("Quest status req - not implemented");
    Ok(vec![])
}
