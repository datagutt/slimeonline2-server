//! Appearance change handlers (outfit, accessories)

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::debug;

use crate::game::PlayerSession;
use crate::protocol::{MessageWriter, MessageType};
use crate::Server;

/// Handle outfit change (MSG_CHANGE_OUT)
/// Client sends: slot (1 byte) - the inventory slot of the outfit to equip
pub async fn handle_change_outfit(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let slot = payload[0];

    // Validate slot (1-9, or 0 to unequip)
    if slot > 9 {
        return Ok(vec![]);
    }

    let (player_id, room_id, character_id, new_body_id) = {
        let session_guard = session.read().await;
        
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        let player_id = match session_guard.player_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        let character_id = match session_guard.character_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        (player_id, session_guard.room_id, character_id, 0u16)
    };

    // Get the outfit ID from the inventory slot
    let new_body_id = if slot == 0 {
        0 // Unequip
    } else {
        // Look up the outfit from inventory
        let inventory = match crate::db::get_inventory(&server.db, character_id).await? {
            Some(inv) => inv,
            None => return Ok(vec![]),
        };
        let outfits = inventory.outfits();
        outfits[(slot - 1) as usize]
    };

    // Update session
    {
        let mut session_guard = session.write().await;
        session_guard.body_id = new_body_id;
    }

    // Save to database
    crate::db::update_body_id(&server.db, character_id, new_body_id as i16).await?;

    debug!("Player {} changed outfit to {} (slot {})", player_id, new_body_id, slot);

    // Broadcast outfit change to all players in room
    let room_players = server.game_state.get_room_players(room_id).await;
    
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                writer.write_u16(MessageType::ChangeOutfit.id()).write_u16(player_id).write_u16(new_body_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle accessory 1 change (MSG_CHANGE_ACS1)
/// Client sends: slot (1 byte) - the inventory slot of the accessory to equip
pub async fn handle_change_accessory1(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let slot = payload[0];

    // Validate slot (1-9, or 0 to unequip)
    if slot > 9 {
        return Ok(vec![]);
    }

    let (player_id, room_id, character_id) = {
        let session_guard = session.read().await;
        
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        let player_id = match session_guard.player_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        let character_id = match session_guard.character_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        (player_id, session_guard.room_id, character_id)
    };

    // Get the accessory ID from the inventory slot
    let new_acs_id = if slot == 0 {
        0 // Unequip
    } else {
        // Look up the accessory from inventory
        let inventory = match crate::db::get_inventory(&server.db, character_id).await? {
            Some(inv) => inv,
            None => return Ok(vec![]),
        };
        let accessories = inventory.accessories();
        accessories[(slot - 1) as usize]
    };

    // Update session
    {
        let mut session_guard = session.write().await;
        session_guard.acs1_id = new_acs_id;
    }

    // Save to database
    crate::db::update_accessory1_id(&server.db, character_id, new_acs_id as i16).await?;

    debug!("Player {} changed accessory1 to {} (slot {})", player_id, new_acs_id, slot);

    // Broadcast accessory change to all players in room
    let room_players = server.game_state.get_room_players(room_id).await;
    
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                writer.write_u16(MessageType::ChangeAccessory1.id()).write_u16(player_id).write_u16(new_acs_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}

/// Handle accessory 2 change (MSG_CHANGE_ACS2)
/// Client sends: slot (1 byte) - the inventory slot of the accessory to equip
pub async fn handle_change_accessory2(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.is_empty() {
        return Ok(vec![]);
    }

    let slot = payload[0];

    // Validate slot (1-9, or 0 to unequip)
    if slot > 9 {
        return Ok(vec![]);
    }

    let (player_id, room_id, character_id) = {
        let session_guard = session.read().await;
        
        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        let player_id = match session_guard.player_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        let character_id = match session_guard.character_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        (player_id, session_guard.room_id, character_id)
    };

    // Get the accessory ID from the inventory slot
    let new_acs_id = if slot == 0 {
        0 // Unequip
    } else {
        // Look up the accessory from inventory
        let inventory = match crate::db::get_inventory(&server.db, character_id).await? {
            Some(inv) => inv,
            None => return Ok(vec![]),
        };
        let accessories = inventory.accessories();
        accessories[(slot - 1) as usize]
    };

    // Update session
    {
        let mut session_guard = session.write().await;
        session_guard.acs2_id = new_acs_id;
    }

    // Save to database
    crate::db::update_accessory2_id(&server.db, character_id, new_acs_id as i16).await?;

    debug!("Player {} changed accessory2 to {} (slot {})", player_id, new_acs_id, slot);

    // Broadcast accessory change to all players in room
    let room_players = server.game_state.get_room_players(room_id).await;
    
    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let mut writer = MessageWriter::new();
                writer.write_u16(MessageType::ChangeAccessory2.id()).write_u16(player_id).write_u16(new_acs_id);
                other_session.write().await.queue_message(writer.into_bytes());
            }
        }
    }

    Ok(vec![])
}
