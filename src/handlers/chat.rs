//! Chat, emote, and action message handlers

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::Server;
use crate::anticheat::HackType;
use crate::game::PlayerSession;
use crate::protocol::{ChatMessage, MessageReader, MessageType, MessageWriter};
use crate::rate_limit::ActionType;
use crate::validation::{sanitize_string, validate_chat_message};

/// Handle chat message
pub async fn handle_chat(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let chat = ChatMessage::parse(&mut reader)?;

    // Get session info for rate limiting
    let (player_id, room_id, session_id, username) = {
        let session_guard = session.read().await;

        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        let player_id = match session_guard.player_id {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        (
            player_id,
            session_guard.room_id,
            session_guard.session_id,
            session_guard.username.clone().unwrap_or_default(),
        )
    };

    // Rate limit chat messages
    let rate_result = server
        .rate_limiter
        .check_player(session_id.as_u128() as u64, ActionType::Chat)
        .await;

    if !rate_result.is_allowed() {
        warn!("Chat rate limited for player {}", username);
        return Ok(vec![]);
    }

    // Get max chat length from config
    let max_chat_length = server.game_config.game.limits.max_chat_length;

    // Validate and sanitize message
    let message = match validate_chat_message(&chat.message, max_chat_length) {
        Ok(msg) => msg.to_string(),
        Err(e) => {
            warn!("Invalid chat message from {}: {}", username, e.message);
            // Sanitize and use anyway if it's just too long
            if chat.message.len() > max_chat_length {
                sanitize_string(&chat.message, max_chat_length)
            } else {
                return Ok(vec![]);
            }
        }
    };

    if message.is_empty() {
        return Ok(vec![]);
    }

    info!("[CHAT] {}: {}", username, message);

    // Broadcast to all players in room (including sender)
    let room_players = server.game_state.get_room_players(room_id).await;

    for other_player_id in room_players {
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id)
            && let Some(other_handle) = server.sessions.get(&other_session_id)
        {
            let mut writer = MessageWriter::new();
            ChatMessage::write_broadcast(&mut writer, player_id, &message);
            other_handle.queue_message(writer.into_bytes()).await;
        }
    }

    Ok(vec![])
}

/// Handle typing indicator
pub async fn handle_typing(
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let (player_id, room_id) = {
        let session_guard = session.read().await;

        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        match session_guard.player_id {
            Some(id) => (id, session_guard.room_id),
            None => return Ok(vec![]),
        }
    };

    // Broadcast typing indicator to other players in room
    let room_players = server.game_state.get_room_players(room_id).await;

    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id)
            && let Some(other_handle) = server.sessions.get(&other_session_id)
        {
            let mut writer = MessageWriter::new();
            writer
                .write_u16(MessageType::PlayerTyping.id())
                .write_u16(player_id);
            other_handle.queue_message(writer.into_bytes()).await;
        }
    }

    Ok(vec![])
}

/// Dice emote ID - server generates random result 1-6
const DICE_EMOTE_ID: u8 = 13;

/// Handle MSG_EMOTE (23, port of `case_msg_emote.gml`): the client sends the
/// shortcut SLOT (1-5), never an emote id. The server resolves the slot against
/// the character's inventory (`emote_1..emote_5`) and broadcasts
/// `[u16 pid][u8 emote_id]` to the room. The dice emote (13) also carries the
/// rolled result: the room gets `[pid][13][u8 roll 1-6]` and the sender gets
/// MSG_EMOTE_DICE (93) `[u8 roll]` to resolve its local waiting state.
pub async fn handle_emote(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    use rand::Rng;

    if payload.is_empty() {
        return Ok(vec![]);
    }

    let slot = payload[0];

    let (player_id, room_id, character_id, account_id, ip_address) = {
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

        (
            player_id,
            session_guard.room_id,
            character_id,
            session_guard.account_id,
            session_guard.ip_address.clone(),
        )
    };

    let report = |description: &'static str| {
        let server = Arc::clone(server);
        let ip_address = ip_address.clone();
        async move {
            if let Some(acc_id) = account_id {
                let _ = server
                    .anticheat
                    .report_hack(
                        &server.db,
                        acc_id,
                        Some(character_id),
                        HackType::SuspiciousActivity,
                        description,
                        Some(&ip_address),
                        None,
                        &server.game_config.game.anticheat,
                    )
                    .await;
            }
        }
    };

    if !(1..=5).contains(&slot) {
        warn!("HACK: player {} sent emote from slot {}", player_id, slot);
        report("Tried to send an emote from a Slot smaller then 1 or higher then 5").await;
        return Ok(vec![]);
    }

    let inventory = match crate::db::get_inventory(&server.db, character_id).await? {
        Some(inv) => inv,
        None => return Ok(vec![]),
    };
    let emote_id = inventory.emotes()[(slot - 1) as usize];

    if emote_id == 0 {
        warn!("HACK: player {} sent emote from empty slot {}", player_id, slot);
        report("Tried to send an emote from a Slot where he doesn't have an emote").await;
        return Ok(vec![]);
    }

    let dice_roll = if emote_id == DICE_EMOTE_ID {
        let mut rng = rand::thread_rng();
        Some(rng.gen_range(1u8..=6))
    } else {
        None
    };

    // The original sends 'same room' with no sender exclusion; the GM client
    // ignores its own pid (it is not in p_list), so the echo is harmless and
    // kept for wire fidelity.
    let room_players = server.game_state.get_room_players(room_id).await;

    for other_player_id in room_players {
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id)
            && let Some(other_handle) = server.sessions.get(&other_session_id)
        {
            let mut writer = MessageWriter::new();
            writer
                .write_u16(MessageType::Emote.id())
                .write_u16(player_id)
                .write_u8(emote_id);
            if let Some(roll) = dice_roll {
                writer.write_u8(roll);
            }
            other_handle.queue_message(writer.into_bytes()).await;
        }
    }

    if let Some(roll) = dice_roll {
        let mut writer = MessageWriter::new();
        writer
            .write_u16(MessageType::EmoteDice.id())
            .write_u8(roll);
        return Ok(vec![writer.into_bytes()]);
    }

    Ok(vec![])
}

/// Handle MSG_ACTION (12, port of `case_msg_action.gml`): the sender got hurt.
///
/// Wire: `[u16 x][u16 y][u16 hurtdir][i16 hsp*100][i16 vsp*100]`. The server
/// stores the position and relays the fields with the sender's pid prefixed to
/// everyone else in the room (receivers clear that player's inputs and apply
/// the knockback speeds divided by 100).
pub async fn handle_action(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let (Ok(x), Ok(y), Ok(hurtdir), Ok(hsp), Ok(vsp)) = (
        reader.read_u16(),
        reader.read_u16(),
        reader.read_u16(),
        reader.read_i16(),
        reader.read_i16(),
    ) else {
        return Ok(vec![]);
    };

    let (player_id, room_id) = {
        let mut session_guard = session.write().await;

        if !session_guard.is_authenticated {
            return Ok(vec![]);
        }

        let Some(id) = session_guard.player_id else {
            return Ok(vec![]);
        };
        session_guard.x = x;
        session_guard.y = y;
        (id, session_guard.room_id)
    };

    // Relay to everyone else in the room.
    let room_players = server.game_state.get_room_players(room_id).await;

    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }

        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id)
            && let Some(other_handle) = server.sessions.get(&other_session_id)
        {
            let mut writer = MessageWriter::new();
            writer
                .write_u16(MessageType::Action.id())
                .write_u16(player_id)
                .write_u16(x)
                .write_u16(y)
                .write_u16(hurtdir)
                .write_i16(hsp)
                .write_i16(vsp);
            other_handle.queue_message(writer.into_bytes()).await;
        }
    }

    Ok(vec![])
}

/// Handle new player response (client sending their position)
pub async fn handle_new_player_response(
    payload: &[u8],
    _server: &Arc<Server>,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    if payload.len() < 6 {
        return Ok(vec![]);
    }

    let mut reader = MessageReader::new(payload);
    let _target_pid = reader.read_u16()?;
    let _our_x = reader.read_u16()?;
    let _our_y = reader.read_u16()?;

    // This is just acknowledgment from client, no response needed
    Ok(vec![])
}
