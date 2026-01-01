//! Player management admin endpoints

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::admin::{verify_api_key, AdminAction, AdminState, ApiResponse, InventoryCategory, PointsMode};
use crate::db;

#[derive(Serialize)]
pub struct OnlinePlayer {
    pub username: String,
    pub player_id: u16,
    pub room_id: u16,
    pub x: u16,
    pub y: u16,
    pub points: u32,
    pub is_moderator: bool,
}

/// GET /api/players - List all online players
pub async fn list_online(
    headers: HeaderMap,
    State(state): State<Arc<AdminState>>,
) -> Result<Json<ApiResponse<Vec<OnlinePlayer>>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let mut players = Vec::new();

    for session_ref in state.sessions.iter() {
        if let Ok(session) = session_ref.value().try_read() {
            if session.is_authenticated {
                if let Some(username) = &session.username {
                    players.push(OnlinePlayer {
                        username: username.clone(),
                        player_id: session.player_id.unwrap_or(0),
                        room_id: session.room_id,
                        x: session.x,
                        y: session.y,
                        points: session.points,
                        is_moderator: session.is_moderator,
                    });
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(players)))
}

#[derive(Serialize)]
pub struct PlayerInfo {
    // Account info
    pub account_id: i64,
    pub username: String,
    pub created_at: String,
    pub last_login: Option<String>,
    pub is_banned: bool,
    pub ban_reason: Option<String>,
    // Character info
    pub character_id: i64,
    pub x: i16,
    pub y: i16,
    pub room_id: i16,
    pub body_id: i16,
    pub acs1_id: i16,
    pub acs2_id: i16,
    pub points: i64,
    pub bank_balance: i64,
    pub is_moderator: bool,
    pub clan_id: Option<i64>,
    // Online status
    pub is_online: bool,
    pub current_room: Option<u16>,
    pub current_x: Option<u16>,
    pub current_y: Option<u16>,
    // Inventory
    pub items: [u16; 9],
    pub outfits: [u16; 9],
    pub accessories: [u16; 9],
    pub tools: [u8; 9],
    pub emotes: [u8; 5],
}

/// GET /api/players/:username - Get detailed player info
pub async fn get_info(
    headers: HeaderMap,
    Path(username): Path<String>,
    State(state): State<Arc<AdminState>>,
) -> Result<Json<ApiResponse<PlayerInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let username_lower = username.to_lowercase();

    // Get account
    let account = match db::find_account_by_username(&state.db, &username_lower).await {
        Ok(Some(acc)) => acc,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!("Player '{}' not found", username))),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Database error: {}", e))),
            ))
        }
    };

    // Get character
    let character = match db::find_character_by_account(&state.db, account.id).await {
        Ok(Some(char)) => char,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("Player has no character")),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Database error: {}", e))),
            ))
        }
    };

    // Get inventory
    let inventory = match db::get_inventory(&state.db, character.id).await {
        Ok(Some(inv)) => inv,
        Ok(None) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Player has no inventory")),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Database error: {}", e))),
            ))
        }
    };

    // Check if online and get current state
    let (is_online, current_room, current_x, current_y) = {
        let mut online_info = (false, None, None, None);
        for session_ref in state.sessions.iter() {
            if let Ok(session) = session_ref.value().try_read() {
                if session.username.as_deref() == Some(&username_lower) && session.is_authenticated {
                    online_info = (true, Some(session.room_id), Some(session.x), Some(session.y));
                    break;
                }
            }
        }
        online_info
    };

    let info = PlayerInfo {
        account_id: account.id,
        username: account.username,
        created_at: account.created_at,
        last_login: account.last_login,
        is_banned: account.is_banned,
        ban_reason: account.ban_reason,
        character_id: character.id,
        x: character.x,
        y: character.y,
        room_id: character.room_id,
        body_id: character.body_id,
        acs1_id: character.acs1_id,
        acs2_id: character.acs2_id,
        points: character.points,
        bank_balance: character.bank_balance,
        is_moderator: character.is_moderator,
        clan_id: character.clan_id,
        is_online,
        current_room,
        current_x,
        current_y,
        items: inventory.items(),
        outfits: inventory.outfits(),
        accessories: inventory.accessories(),
        tools: inventory.tools(),
        emotes: inventory.emotes(),
    };

    Ok(Json(ApiResponse::success(info)))
}

#[derive(Deserialize)]
pub struct KickRequest {
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Serialize)]
pub struct KickResponse {
    pub kicked: bool,
    pub was_online: bool,
}

/// POST /api/players/:username/kick - Kick a player
pub async fn kick(
    headers: HeaderMap,
    Path(username): Path<String>,
    State(state): State<Arc<AdminState>>,
    Json(req): Json<KickRequest>,
) -> Result<Json<ApiResponse<KickResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let username_lower = username.to_lowercase();

    // Check if player is online
    let is_online = state.sessions.iter().any(|s| {
        s.value()
            .try_read()
            .map(|s| s.username.as_deref() == Some(&username_lower) && s.is_authenticated)
            .unwrap_or(false)
    });

    if !is_online {
        return Ok(Json(ApiResponse::success(KickResponse {
            kicked: false,
            was_online: false,
        })));
    }

    // Send kick action to game loop
    if state
        .action_tx
        .send(AdminAction::KickPlayer {
            username: username_lower,
            reason: req.reason,
        })
        .await
        .is_err()
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("Failed to queue kick action")),
        ));
    }

    Ok(Json(ApiResponse::success(KickResponse {
        kicked: true,
        was_online: true,
    })))
}

#[derive(Deserialize)]
pub struct BanRequest {
    pub ban_type: String, // "ip", "mac", "account"
    pub reason: String,
    #[serde(default)]
    pub duration_hours: Option<u32>, // None = permanent
    #[serde(default = "default_true")]
    pub kick: bool, // Also kick if online (default true)
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
pub struct BanResponse {
    pub banned: bool,
    pub ban_id: i64,
    pub kicked: bool,
}

/// POST /api/players/:username/ban - Ban a player
pub async fn ban(
    headers: HeaderMap,
    Path(username): Path<String>,
    State(state): State<Arc<AdminState>>,
    Json(req): Json<BanRequest>,
) -> Result<Json<ApiResponse<BanResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let username_lower = username.to_lowercase();

    // Validate ban type
    if !["ip", "mac", "account"].contains(&req.ban_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Invalid ban_type. Must be 'ip', 'mac', or 'account'")),
        ));
    }

    // Get account info
    let account = match db::find_account_by_username(&state.db, &username_lower).await {
        Ok(Some(acc)) => acc,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!("Player '{}' not found", username))),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Database error: {}", e))),
            ))
        }
    };

    // Determine the ban value based on type
    let ban_value = match req.ban_type.as_str() {
        "account" => username_lower.clone(),
        "mac" => account.mac_address.clone(),
        "ip" => {
            // Try to get IP from online session
            let ip = state.sessions.iter().find_map(|s| {
                s.value().try_read().ok().and_then(|session| {
                    if session.username.as_deref() == Some(&username_lower) {
                        Some(session.ip_address.clone())
                    } else {
                        None
                    }
                })
            });
            match ip {
                Some(ip) => ip,
                None => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(ApiResponse::error("Cannot ban by IP: player is not online")),
                    ))
                }
            }
        }
        _ => unreachable!(),
    };

    // Calculate expiry
    let expires_at = req.duration_hours.and_then(|hours| {
        chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(hours as i64))
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
    });

    // Insert ban record
    let ban_id = match sqlx::query(
        r#"
        INSERT INTO bans (ban_type, value, reason, banned_by, expires_at)
        VALUES (?, ?, ?, 'admin_api', ?)
        ON CONFLICT (ban_type, value) DO UPDATE SET
            reason = excluded.reason,
            banned_by = excluded.banned_by,
            banned_at = datetime('now'),
            expires_at = excluded.expires_at
        "#,
    )
    .bind(&req.ban_type)
    .bind(&ban_value)
    .bind(&req.reason)
    .bind(&expires_at)
    .execute(&state.db)
    .await
    {
        Ok(result) => result.last_insert_rowid(),
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Failed to create ban: {}", e))),
            ))
        }
    };

    // Also update account banned flag if account ban
    if req.ban_type == "account" {
        let _ = sqlx::query("UPDATE accounts SET is_banned = 1, ban_reason = ? WHERE username = ?")
            .bind(&req.reason)
            .bind(&username_lower)
            .execute(&state.db)
            .await;
    }

    // Optionally kick the player
    let kicked = if req.kick {
        let is_online = state.sessions.iter().any(|s| {
            s.value()
                .try_read()
                .map(|s| s.username.as_deref() == Some(&username_lower) && s.is_authenticated)
                .unwrap_or(false)
        });

        if is_online {
            let _ = state
                .action_tx
                .send(AdminAction::KickPlayer {
                    username: username_lower,
                    reason: Some(format!("Banned: {}", req.reason)),
                })
                .await;
            true
        } else {
            false
        }
    } else {
        false
    };

    Ok(Json(ApiResponse::success(BanResponse {
        banned: true,
        ban_id,
        kicked,
    })))
}

#[derive(Deserialize)]
pub struct TeleportRequest {
    pub room_id: u16,
    #[serde(default = "default_coord")]
    pub x: u16,
    #[serde(default = "default_coord")]
    pub y: u16,
}

fn default_coord() -> u16 {
    100
}

#[derive(Serialize)]
pub struct TeleportResponse {
    pub teleported: bool,
    pub was_online: bool,
}

/// POST /api/players/:username/teleport - Teleport a player
pub async fn teleport(
    headers: HeaderMap,
    Path(username): Path<String>,
    State(state): State<Arc<AdminState>>,
    Json(req): Json<TeleportRequest>,
) -> Result<Json<ApiResponse<TeleportResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let username_lower = username.to_lowercase();

    // Check if player is online
    let is_online = state.sessions.iter().any(|s| {
        s.value()
            .try_read()
            .map(|s| s.username.as_deref() == Some(&username_lower) && s.is_authenticated)
            .unwrap_or(false)
    });

    if !is_online {
        // Update DB position for offline player
        match db::find_account_by_username(&state.db, &username_lower).await {
            Ok(Some(account)) => {
                if let Ok(Some(character)) = db::find_character_by_account(&state.db, account.id).await {
                    let _ = db::update_position(
                        &state.db,
                        character.id,
                        req.x as i16,
                        req.y as i16,
                        req.room_id as i16,
                    )
                    .await;
                }
            }
            _ => {}
        }

        return Ok(Json(ApiResponse::success(TeleportResponse {
            teleported: true,
            was_online: false,
        })));
    }

    // Send teleport action for online player
    if state
        .action_tx
        .send(AdminAction::TeleportPlayer {
            username: username_lower,
            room_id: req.room_id,
            x: req.x,
            y: req.y,
        })
        .await
        .is_err()
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("Failed to queue teleport action")),
        ));
    }

    Ok(Json(ApiResponse::success(TeleportResponse {
        teleported: true,
        was_online: true,
    })))
}

#[derive(Deserialize)]
pub struct SetPointsRequest {
    pub points: i64,
    #[serde(default = "default_mode")]
    pub mode: String, // "set", "add", "subtract"
}

fn default_mode() -> String {
    "set".to_string()
}

#[derive(Serialize)]
pub struct SetPointsResponse {
    pub queued: bool,
}

/// POST /api/players/:username/points - Set or modify player points
/// The actual update is handled by the game loop to ensure proper client notification
pub async fn set_points(
    headers: HeaderMap,
    Path(username): Path<String>,
    State(state): State<Arc<AdminState>>,
    Json(req): Json<SetPointsRequest>,
) -> Result<Json<ApiResponse<SetPointsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let mode = PointsMode::from_str(&req.mode).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Invalid mode. Use 'set', 'add', or 'subtract'")),
        )
    })?;

    // Validate points range
    if req.points < 0 && mode == PointsMode::Set {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Points cannot be negative for 'set' mode")),
        ));
    }

    // Send to game loop for processing
    if state
        .action_tx
        .send(AdminAction::SetPoints {
            username: username.to_lowercase(),
            points: req.points,
            mode,
        })
        .await
        .is_err()
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("Failed to queue points action")),
        ));
    }

    Ok(Json(ApiResponse::success(SetPointsResponse { queued: true })))
}

#[derive(Deserialize)]
pub struct SetBankRequest {
    pub balance: i64,
    #[serde(default = "default_mode")]
    pub mode: String, // "set", "add", "subtract"
}

#[derive(Serialize)]
pub struct SetBankResponse {
    pub queued: bool,
}

/// POST /api/players/:username/bank - Set or modify player bank balance
pub async fn set_bank(
    headers: HeaderMap,
    Path(username): Path<String>,
    State(state): State<Arc<AdminState>>,
    Json(req): Json<SetBankRequest>,
) -> Result<Json<ApiResponse<SetBankResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let mode = PointsMode::from_str(&req.mode).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Invalid mode. Use 'set', 'add', or 'subtract'")),
        )
    })?;

    if state
        .action_tx
        .send(AdminAction::SetBank {
            username: username.to_lowercase(),
            balance: req.balance,
            mode,
        })
        .await
        .is_err()
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("Failed to queue bank action")),
        ));
    }

    Ok(Json(ApiResponse::success(SetBankResponse { queued: true })))
}

#[derive(Deserialize)]
pub struct SetInventorySlotRequest {
    pub category: String, // "item", "outfit", "accessory", "tool", "emote"
    pub slot: u8,         // 1-9 (or 1-5 for emotes)
    pub item_id: u16,     // item/outfit/accessory/tool/emote ID (0 = empty)
}

#[derive(Serialize)]
pub struct SetInventorySlotResponse {
    pub queued: bool,
}

/// POST /api/players/:username/inventory - Set an inventory slot
pub async fn set_inventory_slot(
    headers: HeaderMap,
    Path(username): Path<String>,
    State(state): State<Arc<AdminState>>,
    Json(req): Json<SetInventorySlotRequest>,
) -> Result<Json<ApiResponse<SetInventorySlotResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let category = InventoryCategory::from_str(&req.category).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "Invalid category. Use 'item', 'outfit', 'accessory', 'tool', or 'emote'",
            )),
        )
    })?;

    // Validate slot range
    if req.slot < 1 || req.slot > category.max_slot() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "Invalid slot. Must be 1-{} for {:?}",
                category.max_slot(),
                category
            ))),
        ));
    }

    if state
        .action_tx
        .send(AdminAction::GiveItem {
            username: username.to_lowercase(),
            category,
            slot: req.slot,
            item_id: req.item_id,
        })
        .await
        .is_err()
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("Failed to queue inventory action")),
        ));
    }

    Ok(Json(ApiResponse::success(SetInventorySlotResponse {
        queued: true,
    })))
}

#[derive(Deserialize)]
pub struct SetModeratorRequest {
    pub is_moderator: bool,
}

#[derive(Serialize)]
pub struct SetModeratorResponse {
    pub updated: bool,
}

/// POST /api/players/:username/moderator - Set moderator status
/// This is a DB-only operation (no client message needed)
pub async fn set_moderator(
    headers: HeaderMap,
    Path(username): Path<String>,
    State(state): State<Arc<AdminState>>,
    Json(req): Json<SetModeratorRequest>,
) -> Result<Json<ApiResponse<SetModeratorResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let username_lower = username.to_lowercase();

    // Get character
    let account = match db::find_account_by_username(&state.db, &username_lower).await {
        Ok(Some(acc)) => acc,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!("Player '{}' not found", username))),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Database error: {}", e))),
            ))
        }
    };

    let character = match db::find_character_by_account(&state.db, account.id).await {
        Ok(Some(char)) => char,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("Player has no character")),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Database error: {}", e))),
            ))
        }
    };

    // Update is_moderator flag in DB
    if let Err(e) = sqlx::query("UPDATE characters SET is_moderator = ? WHERE id = ?")
        .bind(req.is_moderator)
        .bind(character.id)
        .execute(&state.db)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Failed to update moderator status: {}", e))),
        ));
    }

    // Update online session if present
    for session_ref in state.sessions.iter() {
        if let Ok(mut session) = session_ref.value().try_write() {
            if session.username.as_deref() == Some(&username_lower) && session.is_authenticated {
                session.is_moderator = req.is_moderator;
                break;
            }
        }
    }

    Ok(Json(ApiResponse::success(SetModeratorResponse {
        updated: true,
    })))
}
