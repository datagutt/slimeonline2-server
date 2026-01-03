//! Clan system handlers
//!
//! Handles clan creation, management, and member operations.
//! - MSG_CLAN_CREATE (126) - Create a new clan
//! - MSG_CLAN_DISSOLVE (127) - Dissolve the clan (leader only)
//! - MSG_CLAN_INVITE (128) - Accept/decline clan invite
//! - MSG_CLAN_LEAVE (129) - Leave the clan
//! - MSG_CLAN_INFO (130) - Clan information requests/updates
//! - MSG_CLAN_ADMIN (131) - Admin actions (kick, invite, colors, info, news)

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::db;
use crate::game::{PendingClanInvite, PlayerSession};
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::Server;

/// Clan invite cooldown in seconds (15s between invites to the same player)
const CLAN_INVITE_COOLDOWN_SECS: u64 = 15;

// ============================================================================
// MSG_CLAN_CREATE (126)
// ============================================================================

/// Handle MSG_CLAN_CREATE (126)
/// Create a new clan with required items and points
pub async fn handle_clan_create(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let clan_name = reader.read_string()?;

    let (character_id, player_id, current_clan_id, points) = {
        let session_guard = session.read().await;
        (
            session_guard.character_id,
            session_guard.player_id,
            session_guard.clan_id,
            session_guard.points,
        )
    };

    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    let pid = match player_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Check: player must not already be in a clan
    if current_clan_id.is_some() {
        debug!("Clan create failed: player {} already in a clan", char_id);
        return Ok(vec![]);
    }

    // Validate clan name length
    let config = &server.game_config.clans;
    if clan_name.len() < config.limits.min_name_length
        || clan_name.len() > config.limits.max_name_length
    {
        debug!(
            "Clan create failed: name '{}' invalid length (need {}-{})",
            clan_name, config.limits.min_name_length, config.limits.max_name_length
        );
        return Ok(vec![build_clan_create_error(1)]); // 1 = name error
    }

    // Check if clan name is already taken
    if db::is_clan_name_taken(&server.db, &clan_name)
        .await
        .unwrap_or(true)
    {
        debug!("Clan create failed: name '{}' already exists", clan_name);
        return Ok(vec![build_clan_create_error(1)]); // 1 = name in use
    }

    // Check player has enough points
    let creation_cost = config.creation.cost;
    if points < creation_cost {
        debug!(
            "Clan create failed: player {} has {} points, need {}",
            char_id, points, creation_cost
        );
        return Ok(vec![]);
    }

    // Check player has required items (Proof of Nature + Proof of Earth)
    let inventory = match db::get_inventory(&server.db, char_id).await {
        Ok(Some(inv)) => inv,
        Ok(None) => {
            warn!("No inventory found for clan create");
            return Ok(vec![]);
        }
        Err(e) => {
            warn!("Failed to get inventory for clan create: {}", e);
            return Ok(vec![]);
        }
    };

    let items = inventory.items();

    // Find slots with required items
    let mut item_slots: Vec<(usize, u16)> = Vec::new(); // (slot_index, item_id)
    for required_item in &config.creation.required_items {
        let mut found = false;
        for (slot_idx, &item_id) in items.iter().enumerate() {
            // Skip already used slots
            if item_slots.iter().any(|(s, _)| *s == slot_idx) {
                continue;
            }
            if item_id == *required_item {
                item_slots.push((slot_idx, *required_item));
                found = true;
                break;
            }
        }
        if !found {
            debug!(
                "Clan create failed: player {} missing required item {}",
                char_id, required_item
            );
            return Ok(vec![]);
        }
    }

    // All checks passed - create the clan!

    // 1. Deduct points
    let new_points = points - creation_cost;
    if let Err(e) = db::update_points(&server.db, char_id, new_points as i64).await {
        warn!("Failed to deduct points for clan create: {}", e);
        return Ok(vec![]);
    }

    // 2. Remove required items from inventory
    for (slot_idx, _) in &item_slots {
        let slot_num = (*slot_idx + 1) as u8;
        if let Err(e) = db::update_item_slot(&server.db, char_id, slot_num, 0).await {
            warn!(
                "Failed to remove item from slot {} for clan create: {}",
                slot_num, e
            );
            // Continue anyway, clan is being created
        }
    }

    // 3. Create the clan
    let clan_id = match db::create_clan(
        &server.db,
        &clan_name,
        char_id,
        config.limits.initial_member_slots,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            warn!("Failed to create clan '{}': {}", clan_name, e);
            // TODO: Refund points/items on failure?
            return Ok(vec![]);
        }
    };

    // 4. Update session state
    {
        let mut session_guard = session.write().await;
        session_guard.clan_id = Some(clan_id);
        session_guard.is_clan_leader = true;
        session_guard.points = new_points;
    }

    info!(
        "Player {} created clan '{}' (id={})",
        char_id, clan_name, clan_id
    );

    // Build response messages
    let mut responses = Vec::new();

    // Response to creator (MSG_CLAN_INFO type 1)
    responses.push(build_clan_info_self(clan_id, true, false));

    // Broadcast to all players (MSG_CLAN_INFO type 2)
    let broadcast = build_clan_info_broadcast(pid, clan_id);
    broadcast_to_all_players(server, pid, broadcast).await;

    // Also send points update
    responses.push(build_points_update(new_points));

    Ok(responses)
}

/// Build clan create error response
fn build_clan_create_error(error_code: u8) -> Vec<u8> {
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::ClanCreate.id())
        .write_u8(error_code);
    writer.into_bytes()
}

/// Build MSG_CLAN_INFO type 1 (self notification - joined clan)
fn build_clan_info_self(clan_id: i64, is_leader: bool, has_base: bool) -> Vec<u8> {
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::ClanInfo.id())
        .write_u8(1) // type 1 = self joined clan
        .write_u16(clan_id as u16)
        .write_u8(if is_leader { 1 } else { 0 })
        .write_u8(if has_base { 1 } else { 0 });
    writer.into_bytes()
}

/// Build MSG_CLAN_INFO type 2 (broadcast - player is in clan)
fn build_clan_info_broadcast(player_id: u16, clan_id: i64) -> Vec<u8> {
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::ClanInfo.id())
        .write_u8(2) // type 2 = broadcast
        .write_u16(player_id)
        .write_u16(clan_id as u16);
    writer.into_bytes()
}

/// Build points update message using common utility
fn build_points_update(points: u32) -> Vec<u8> {
    crate::protocol::build_points_update(points, false)
}

// ============================================================================
// MSG_CLAN_DISSOLVE (127)
// ============================================================================

/// Handle MSG_CLAN_DISSOLVE (127)
/// Leader dissolves the clan, removing all members
pub async fn handle_clan_dissolve(
    _payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let (character_id, player_id, clan_id, is_leader) = {
        let session_guard = session.read().await;
        (
            session_guard.character_id,
            session_guard.player_id,
            session_guard.clan_id,
            session_guard.is_clan_leader,
        )
    };

    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    let _pid = match player_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Must be in a clan
    let current_clan_id = match clan_id {
        Some(id) => id,
        None => {
            debug!("Clan dissolve failed: player {} not in a clan", char_id);
            return Ok(vec![]);
        }
    };

    // Must be the leader
    if !is_leader {
        debug!("Clan dissolve failed: player {} is not the leader", char_id);
        return Ok(vec![]);
    }

    // Double-check with database
    if !db::is_clan_leader(&server.db, char_id, current_clan_id)
        .await
        .unwrap_or(false)
    {
        warn!("Clan dissolve: session says leader but DB disagrees");
        return Ok(vec![]);
    }

    // Get all members before dissolving (to notify them)
    let members = db::get_clan_members(&server.db, current_clan_id)
        .await
        .unwrap_or_default();

    // Dissolve the clan
    if let Err(e) = db::dissolve_clan(&server.db, current_clan_id).await {
        warn!("Failed to dissolve clan {}: {}", current_clan_id, e);
        return Ok(vec![]);
    }

    info!("Player {} dissolved clan {}", char_id, current_clan_id);

    // Update session state
    {
        let mut session_guard = session.write().await;
        session_guard.clan_id = None;
        session_guard.is_clan_leader = false;
        session_guard.has_clan_base = false;
    }

    // Notify all members that they're no longer in a clan (MSG_CLAN_INFO type 6)
    let left_msg = build_clan_info_left();
    for member in &members {
        // Find the member's session and send them the message
        if let Some(handle) = find_session_by_character_id(server, member.character_id).await {
            {
                let mut session_guard = handle.session.write().await;
                session_guard.clan_id = None;
                session_guard.is_clan_leader = false;
                session_guard.has_clan_base = false;
            }
            handle.queue_message(left_msg.clone()).await;
        }
    }

    // Broadcast to all players that these members are no longer in a clan
    for member in &members {
        if let Some(handle) = find_session_by_character_id(server, member.character_id).await {
            let pid = handle.session.read().await.player_id;
            if let Some(member_pid) = pid {
                let broadcast = build_clan_info_broadcast(member_pid, 0); // clan_id 0 = no clan
                broadcast_to_all_players(server, member_pid, broadcast).await;
            }
        }
    }

    Ok(vec![build_clan_info_left()])
}

/// Build MSG_CLAN_INFO type 6 (left/kicked from clan)
fn build_clan_info_left() -> Vec<u8> {
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::ClanInfo.id()).write_u8(6); // type 6 = left clan
    writer.into_bytes()
}

// ============================================================================
// MSG_CLAN_INVITE (128)
// ============================================================================

/// Handle MSG_CLAN_INVITE (128)
/// Accept or decline a clan invitation
pub async fn handle_clan_invite_response(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let response = reader.read_u8()?; // 1=Accept, 2=Decline

    let (character_id, player_id, pending_invite, current_clan_id) = {
        let session_guard = session.read().await;
        (
            session_guard.character_id,
            session_guard.player_id,
            session_guard.pending_clan_invite.clone(),
            session_guard.clan_id,
        )
    };

    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    let pid = match player_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Must have a pending invite
    let invite = match pending_invite {
        Some(inv) => inv,
        None => {
            debug!(
                "Clan invite response: player {} has no pending invite",
                char_id
            );
            return Ok(vec![]);
        }
    };

    // Clear the pending invite
    {
        let mut session_guard = session.write().await;
        session_guard.pending_clan_invite = None;
    }

    if response == 2 {
        // Declined
        debug!(
            "Player {} declined invite to clan {}",
            char_id, invite.clan_name
        );
        return Ok(vec![]);
    }

    // Accept - validate requirements

    // Player must not already be in a clan
    if current_clan_id.is_some() {
        debug!(
            "Clan invite accept failed: player {} already in a clan",
            char_id
        );
        return Ok(vec![]);
    }

    // Clan must still exist
    let clan = match db::get_clan(&server.db, invite.clan_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            debug!(
                "Clan invite accept failed: clan {} no longer exists",
                invite.clan_id
            );
            return Ok(vec![]);
        }
        Err(e) => {
            warn!("Failed to get clan {}: {}", invite.clan_id, e);
            return Ok(vec![]);
        }
    };

    // Clan must have room for new member
    let member_count = db::get_clan_member_count(&server.db, invite.clan_id)
        .await
        .unwrap_or(999);
    if member_count >= clan.max_members {
        debug!(
            "Clan invite accept failed: clan {} is full ({}/{})",
            invite.clan_id, member_count, clan.max_members
        );
        return Ok(vec![]);
    }

    // Add player to clan
    if let Err(e) = db::add_clan_member(&server.db, invite.clan_id, char_id).await {
        warn!(
            "Failed to add player {} to clan {}: {}",
            char_id, invite.clan_id, e
        );
        return Ok(vec![]);
    }

    info!(
        "Player {} joined clan '{}' (id={})",
        char_id, clan.name, invite.clan_id
    );

    // Update session
    {
        let mut session_guard = session.write().await;
        session_guard.clan_id = Some(invite.clan_id);
        session_guard.is_clan_leader = false;
        session_guard.has_clan_base = clan.has_base != 0;
    }

    // Response to joiner (MSG_CLAN_INFO type 1)
    let self_msg = build_clan_info_self(invite.clan_id, false, clan.has_base != 0);

    // Broadcast to all players
    let broadcast = build_clan_info_broadcast(pid, invite.clan_id);
    broadcast_to_all_players(server, pid, broadcast).await;

    Ok(vec![self_msg])
}

// ============================================================================
// MSG_CLAN_LEAVE (129)
// ============================================================================

/// Handle MSG_CLAN_LEAVE (129)
/// Leave the current clan (non-leaders only)
pub async fn handle_clan_leave(
    _payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let (character_id, player_id, clan_id, is_leader) = {
        let session_guard = session.read().await;
        (
            session_guard.character_id,
            session_guard.player_id,
            session_guard.clan_id,
            session_guard.is_clan_leader,
        )
    };

    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    let pid = match player_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Must be in a clan
    let current_clan_id = match clan_id {
        Some(id) => id,
        None => {
            debug!("Clan leave failed: player {} not in a clan", char_id);
            return Ok(vec![]);
        }
    };

    // Leader cannot leave (must dissolve instead)
    if is_leader {
        debug!(
            "Clan leave failed: player {} is the leader (must dissolve)",
            char_id
        );
        return Ok(vec![]);
    }

    // Remove from clan
    if let Err(e) = db::remove_clan_member(&server.db, char_id).await {
        warn!("Failed to remove player {} from clan: {}", char_id, e);
        return Ok(vec![]);
    }

    info!("Player {} left clan {}", char_id, current_clan_id);

    // Update session
    {
        let mut session_guard = session.write().await;
        session_guard.clan_id = None;
        session_guard.is_clan_leader = false;
        session_guard.has_clan_base = false;
    }

    // Broadcast to all players
    let broadcast = build_clan_info_broadcast(pid, 0); // clan_id 0 = no clan
    broadcast_to_all_players(server, pid, broadcast).await;

    Ok(vec![build_clan_info_left()])
}

// ============================================================================
// MSG_CLAN_INFO (130)
// ============================================================================

/// Handle MSG_CLAN_INFO (130)
/// Information requests about clans
pub async fn handle_clan_info(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let info_type = reader.read_u8()?;

    match info_type {
        1 => handle_clan_info_name(&mut reader, server, session).await,
        2 => handle_clan_info_members(server, session).await,
        3 => handle_clan_info_status(&mut reader, server, session).await,
        4 => handle_clan_info_text(server, session).await,
        5 => handle_clan_info_news(server, session).await,
        _ => {
            debug!("Unknown clan info type: {}", info_type);
            Ok(vec![])
        }
    }
}

/// Type 1: Get clan name and colors by clan_id
async fn handle_clan_info_name(
    reader: &mut MessageReader<'_>,
    server: &Arc<Server>,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let clan_id = reader.read_u16()? as i64;

    let clan = match db::get_clan(&server.db, clan_id).await {
        Ok(Some(c)) => c,
        Ok(None) => return Ok(vec![]),
        Err(_) => return Ok(vec![]),
    };

    // Parse colors (stored as RGB packed into i64)
    let inner_r = ((clan.color_inner >> 16) & 0xFF) as u8;
    let inner_g = ((clan.color_inner >> 8) & 0xFF) as u8;
    let inner_b = (clan.color_inner & 0xFF) as u8;
    let outer_r = ((clan.color_outer >> 16) & 0xFF) as u8;
    let outer_g = ((clan.color_outer >> 8) & 0xFF) as u8;
    let outer_b = (clan.color_outer & 0xFF) as u8;

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::ClanInfo.id())
        .write_u8(3) // type 3 = name/colors response
        .write_u16(clan_id as u16)
        .write_string(&clan.name)
        .write_u8(inner_r)
        .write_u8(inner_g)
        .write_u8(inner_b)
        .write_u8(outer_r)
        .write_u8(outer_g)
        .write_u8(outer_b);

    Ok(vec![writer.into_bytes()])
}

/// Type 2: Get member list
async fn handle_clan_info_members(
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let clan_id = session.read().await.clan_id;

    let current_clan_id = match clan_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    let clan = match db::get_clan(&server.db, current_clan_id).await {
        Ok(Some(c)) => c,
        Ok(None) => return Ok(vec![]),
        Err(_) => return Ok(vec![]),
    };

    let members = db::get_clan_members(&server.db, current_clan_id)
        .await
        .unwrap_or_default();

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::ClanInfo.id())
        .write_u8(4) // type 4 = member list
        .write_u8(clan.max_members as u8)
        .write_u8(members.len() as u8);

    // Write leader ID first
    writer.write_u32(clan.leader_id as u32);

    // Write each member (up to max slots)
    for member in &members {
        writer.write_u32(member.character_id as u32);
        writer.write_string(&member.username);
    }

    Ok(vec![writer.into_bytes()])
}

/// Type 3: Get status (points only or full)
async fn handle_clan_info_status(
    reader: &mut MessageReader<'_>,
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let sub_type = reader.read_u8()?; // 1=points only, 2=full

    let clan_id = session.read().await.clan_id;

    let current_clan_id = match clan_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    let clan = match db::get_clan(&server.db, current_clan_id).await {
        Ok(Some(c)) => c,
        Ok(None) => return Ok(vec![]),
        Err(_) => return Ok(vec![]),
    };

    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::ClanInfo.id()).write_u8(5); // type 5 = status response

    if sub_type == 1 {
        // Points only
        writer.write_u32(clan.points as u32);
    } else {
        // Full status
        writer
            .write_u32(clan.points as u32)
            .write_u8(clan.level as u8)
            .write_u8(if clan.has_base != 0 { 1 } else { 0 });
    }

    Ok(vec![writer.into_bytes()])
}

/// Type 4: Get info text (leader only)
async fn handle_clan_info_text(
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let (clan_id, is_leader) = {
        let sg = session.read().await;
        (sg.clan_id, sg.is_clan_leader)
    };

    // Leader only
    if !is_leader {
        return Ok(vec![]);
    }

    let current_clan_id = match clan_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    let clan = match db::get_clan(&server.db, current_clan_id).await {
        Ok(Some(c)) => c,
        Ok(None) => return Ok(vec![]),
        Err(_) => return Ok(vec![]),
    };

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::ClanInfo.id())
        .write_u8(7) // type 7 = info text response
        .write_u8(clan.show_name as u8)
        .write_string(&clan.description.unwrap_or_default());

    Ok(vec![writer.into_bytes()])
}

/// Type 5: Get news
async fn handle_clan_info_news(
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let clan_id = session.read().await.clan_id;

    let current_clan_id = match clan_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    let clan = match db::get_clan(&server.db, current_clan_id).await {
        Ok(Some(c)) => c,
        Ok(None) => return Ok(vec![]),
        Err(_) => return Ok(vec![]),
    };

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::ClanInfo.id())
        .write_u8(8) // type 8 = news response
        .write_string(&clan.news.unwrap_or_default());

    Ok(vec![writer.into_bytes()])
}

// ============================================================================
// MSG_CLAN_ADMIN (131)
// ============================================================================

/// Handle MSG_CLAN_ADMIN (131)
/// Admin actions: kick, invite, colors, info, news
pub async fn handle_clan_admin(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let action = reader.read_u8()?;

    match action {
        1 => handle_admin_kick(&mut reader, server, session).await,
        2 => handle_admin_invite(&mut reader, server, session).await,
        3 => handle_admin_colors(&mut reader, server, session).await,
        4 => handle_admin_info(&mut reader, server, session).await,
        5 => handle_admin_news(&mut reader, server, session).await,
        _ => {
            debug!("Unknown clan admin action: {}", action);
            Ok(vec![])
        }
    }
}

/// Action 1: Kick a member
async fn handle_admin_kick(
    reader: &mut MessageReader<'_>,
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let member_slot = reader.read_u8()?;

    let (character_id, clan_id, is_leader) = {
        let sg = session.read().await;
        (sg.character_id, sg.clan_id, sg.is_clan_leader)
    };

    let _char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Leader only
    if !is_leader {
        return Ok(vec![]);
    }

    let current_clan_id = match clan_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Get members to find who to kick
    let members = db::get_clan_members(&server.db, current_clan_id)
        .await
        .unwrap_or_default();

    if member_slot as usize >= members.len() {
        return Ok(vec![]);
    }

    let kicked_member = &members[member_slot as usize];

    // Remove from clan
    if let Err(e) = db::remove_clan_member(&server.db, kicked_member.character_id).await {
        warn!(
            "Failed to kick member {}: {}",
            kicked_member.character_id, e
        );
        return Ok(vec![]);
    }

    info!(
        "Kicked {} from clan {}",
        kicked_member.username, current_clan_id
    );

    // Notify the kicked player if online
    if let Some(handle) =
        find_session_by_character_id(server, kicked_member.character_id).await
    {
        let pid = {
            let mut session_guard = handle.session.write().await;
            session_guard.clan_id = None;
            session_guard.is_clan_leader = false;
            session_guard.has_clan_base = false;
            session_guard.player_id
        };
        handle.queue_message(build_clan_info_left()).await;

        // Broadcast
        if let Some(pid) = pid {
            let broadcast = build_clan_info_broadcast(pid, 0);
            broadcast_to_all_players(server, pid, broadcast).await;
        }
    }

    Ok(vec![])
}

/// Action 2: Invite a player
async fn handle_admin_invite(
    reader: &mut MessageReader<'_>,
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let target_pid = reader.read_u16()?;

    let (character_id, player_id, clan_id, is_leader) = {
        let sg = session.read().await;
        (sg.character_id, sg.player_id, sg.clan_id, sg.is_clan_leader)
    };

    let _char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    let my_pid = match player_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Leader only
    if !is_leader {
        return Ok(vec![]);
    }

    let current_clan_id = match clan_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Check invite cooldown
    {
        let session_guard = session.read().await;
        if let Some(last_invite) = session_guard.clan_invite_cooldowns.get(&target_pid) {
            if last_invite.elapsed().as_secs() < CLAN_INVITE_COOLDOWN_SECS {
                debug!(
                    "Clan invite cooldown for player {} ({}s remaining)",
                    target_pid,
                    CLAN_INVITE_COOLDOWN_SECS - last_invite.elapsed().as_secs()
                );
                return Ok(vec![]);
            }
        }
    }

    // Update cooldown
    {
        let mut session_guard = session.write().await;
        session_guard
            .clan_invite_cooldowns
            .insert(target_pid, Instant::now());
    }

    // Find target player's session
    let target_handle = match find_session_by_player_id(server, target_pid).await {
        Some(s) => s,
        None => {
            debug!("Clan invite failed: player {} not found", target_pid);
            return Ok(vec![]);
        }
    };

    // Check if target is already in a clan
    {
        let tg = target_handle.session.read().await;
        if tg.clan_id.is_some() {
            debug!(
                "Clan invite failed: player {} already in a clan",
                target_pid
            );
            return Ok(vec![]);
        }
    }

    // Get clan info
    let clan = match db::get_clan(&server.db, current_clan_id).await {
        Ok(Some(c)) => c,
        Ok(None) => return Ok(vec![]),
        Err(_) => return Ok(vec![]),
    };

    // Check if clan has room
    let member_count = db::get_clan_member_count(&server.db, current_clan_id)
        .await
        .unwrap_or(999);
    if member_count >= clan.max_members {
        debug!("Clan invite failed: clan {} is full", current_clan_id);
        return Ok(vec![]);
    }

    // Set pending invite on target
    {
        let mut tg = target_handle.session.write().await;
        tg.pending_clan_invite = Some(PendingClanInvite {
            clan_id: current_clan_id,
            clan_name: clan.name.clone(),
            inviter_id: my_pid,
            invited_at: Instant::now(),
        });
    }
    
    // Send invite message to target
    let invite_msg = build_clan_invite(my_pid, &clan.name);
    target_handle.queue_message(invite_msg).await;

    debug!(
        "Sent clan invite to player {} for clan '{}'",
        target_pid, clan.name
    );

    Ok(vec![])
}

/// Build clan invite message to send to invited player
fn build_clan_invite(inviter_pid: u16, clan_name: &str) -> Vec<u8> {
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::ClanInvite.id())
        .write_u16(inviter_pid)
        .write_string(clan_name);
    writer.into_bytes()
}

/// Action 3: Change clan colors
async fn handle_admin_colors(
    reader: &mut MessageReader<'_>,
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let inner_r = reader.read_u8()?;
    let inner_g = reader.read_u8()?;
    let inner_b = reader.read_u8()?;
    let outer_r = reader.read_u8()?;
    let outer_g = reader.read_u8()?;
    let outer_b = reader.read_u8()?;

    let (clan_id, is_leader) = {
        let sg = session.read().await;
        (sg.clan_id, sg.is_clan_leader)
    };

    // Leader only
    if !is_leader {
        return Ok(vec![]);
    }

    let current_clan_id = match clan_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    // Pack colors into u32
    let inner_color = ((inner_r as u32) << 16) | ((inner_g as u32) << 8) | (inner_b as u32);
    let outer_color = ((outer_r as u32) << 16) | ((outer_g as u32) << 8) | (outer_b as u32);

    if let Err(e) =
        db::update_clan_colors(&server.db, current_clan_id, inner_color, outer_color).await
    {
        warn!("Failed to update clan colors: {}", e);
        return Ok(vec![]);
    }

    debug!("Updated clan {} colors", current_clan_id);

    Ok(vec![])
}

/// Action 4: Update info text
async fn handle_admin_info(
    reader: &mut MessageReader<'_>,
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let show_leader = reader.read_u8()? != 0;
    let info_text = reader.read_string()?;

    let (clan_id, is_leader) = {
        let sg = session.read().await;
        (sg.clan_id, sg.is_clan_leader)
    };

    // Leader only
    if !is_leader {
        return Ok(vec![]);
    }

    let current_clan_id = match clan_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    if let Err(e) = db::update_clan_info(&server.db, current_clan_id, show_leader, &info_text).await
    {
        warn!("Failed to update clan info: {}", e);
        return Ok(vec![]);
    }

    debug!("Updated clan {} info", current_clan_id);

    Ok(vec![])
}

/// Action 5: Update news
async fn handle_admin_news(
    reader: &mut MessageReader<'_>,
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let news_text = reader.read_string()?;

    let (clan_id, is_leader) = {
        let sg = session.read().await;
        (sg.clan_id, sg.is_clan_leader)
    };

    // Leader only
    if !is_leader {
        return Ok(vec![]);
    }

    let current_clan_id = match clan_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    if let Err(e) = db::update_clan_news(&server.db, current_clan_id, &news_text).await {
        warn!("Failed to update clan news: {}", e);
        return Ok(vec![]);
    }

    debug!("Updated clan {} news", current_clan_id);

    Ok(vec![])
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Find a session by character ID, returns cloned Arc
async fn find_session_by_character_id(
    server: &Arc<Server>,
    character_id: i64,
) -> Option<Arc<crate::game::SessionHandle>> {
    for entry in server.sessions.iter() {
        let handle = entry.value().clone();
        let session_guard = handle.session.read().await;
        if session_guard.character_id == Some(character_id) {
            drop(session_guard);
            return Some(handle);
        }
    }
    None
}

/// Find a session by player ID, returns cloned Arc
async fn find_session_by_player_id(
    server: &Arc<Server>,
    player_id: u16,
) -> Option<Arc<crate::game::SessionHandle>> {
    for entry in server.sessions.iter() {
        let handle = entry.value().clone();
        let session_guard = handle.session.read().await;
        if session_guard.player_id == Some(player_id) {
            drop(session_guard);
            return Some(handle);
        }
    }
    None
}

/// Broadcast a message to all connected players except the sender
async fn broadcast_to_all_players(server: &Arc<Server>, sender_pid: u16, message: Vec<u8>) {
    for entry in server.sessions.iter() {
        let handle = entry.value().clone();
        let session_guard = handle.session.read().await;
        if session_guard.player_id != Some(sender_pid) && session_guard.is_authenticated {
            drop(session_guard);
            handle.queue_message(message.clone()).await;
        }
    }
}
