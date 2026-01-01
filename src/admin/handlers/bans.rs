//! Ban management admin endpoints

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::admin::{verify_api_key, AdminState, ApiResponse};

#[derive(Serialize)]
pub struct BanRecord {
    pub id: i64,
    pub ban_type: String,
    pub value: String,
    pub reason: String,
    pub banned_by: Option<String>,
    pub banned_at: String,
    pub expires_at: Option<String>,
    pub is_expired: bool,
}

#[derive(Deserialize, Default)]
pub struct ListBansQuery {
    pub ban_type: Option<String>, // Filter by type: "ip", "mac", "account"
    pub include_expired: Option<bool>,
}

/// GET /api/bans - List all bans
pub async fn list_bans(
    headers: HeaderMap,
    State(state): State<Arc<AdminState>>,
    Query(query): Query<ListBansQuery>,
) -> Result<Json<ApiResponse<Vec<BanRecord>>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let include_expired = query.include_expired.unwrap_or(false);

    let bans: Vec<BanRecord> = if let Some(ban_type) = &query.ban_type {
        if !["ip", "mac", "account"].contains(&ban_type.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error("Invalid ban_type filter")),
            ));
        }

        let query_str = if include_expired {
            "SELECT id, ban_type, value, reason, banned_by, banned_at, expires_at FROM bans WHERE ban_type = ? ORDER BY banned_at DESC"
        } else {
            "SELECT id, ban_type, value, reason, banned_by, banned_at, expires_at FROM bans WHERE ban_type = ? AND (expires_at IS NULL OR expires_at > datetime('now')) ORDER BY banned_at DESC"
        };

        sqlx::query_as::<_, (i64, String, String, String, Option<String>, String, Option<String>)>(query_str)
            .bind(ban_type)
            .fetch_all(&state.db)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(format!("Database error: {}", e))),
                )
            })?
            .into_iter()
            .map(|(id, ban_type, value, reason, banned_by, banned_at, expires_at)| {
                let is_expired = expires_at.as_ref().map(|e| e < &chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or(false);
                BanRecord {
                    id,
                    ban_type,
                    value,
                    reason,
                    banned_by,
                    banned_at,
                    expires_at,
                    is_expired,
                }
            })
            .collect()
    } else {
        let query_str = if include_expired {
            "SELECT id, ban_type, value, reason, banned_by, banned_at, expires_at FROM bans ORDER BY banned_at DESC"
        } else {
            "SELECT id, ban_type, value, reason, banned_by, banned_at, expires_at FROM bans WHERE expires_at IS NULL OR expires_at > datetime('now') ORDER BY banned_at DESC"
        };

        sqlx::query_as::<_, (i64, String, String, String, Option<String>, String, Option<String>)>(query_str)
            .fetch_all(&state.db)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(format!("Database error: {}", e))),
                )
            })?
            .into_iter()
            .map(|(id, ban_type, value, reason, banned_by, banned_at, expires_at)| {
                let is_expired = expires_at.as_ref().map(|e| e < &chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or(false);
                BanRecord {
                    id,
                    ban_type,
                    value,
                    reason,
                    banned_by,
                    banned_at,
                    expires_at,
                    is_expired,
                }
            })
            .collect()
    };

    Ok(Json(ApiResponse::success(bans)))
}

#[derive(Deserialize)]
pub struct CreateBanRequest {
    pub ban_type: String,        // "ip", "mac", "account"
    pub value: String,           // The IP, MAC, or username to ban
    pub reason: String,
    pub duration_hours: Option<u32>, // None = permanent
}

#[derive(Serialize)]
pub struct CreateBanResponse {
    pub id: i64,
}

/// POST /api/bans - Create a new ban directly (not via player)
pub async fn create_ban(
    headers: HeaderMap,
    State(state): State<Arc<AdminState>>,
    Json(req): Json<CreateBanRequest>,
) -> Result<Json<ApiResponse<CreateBanResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    if !["ip", "mac", "account"].contains(&req.ban_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("Invalid ban_type. Must be 'ip', 'mac', or 'account'")),
        ));
    }

    if req.value.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("Ban value cannot be empty")),
        ));
    }

    if req.reason.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("Ban reason cannot be empty")),
        ));
    }

    let expires_at = req.duration_hours.and_then(|hours| {
        chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(hours as i64))
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
    });

    let result = sqlx::query(
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
    .bind(&req.value)
    .bind(&req.reason)
    .bind(&expires_at)
    .execute(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("Failed to create ban: {}", e))),
        )
    })?;

    // If it's an account ban, also update the account's is_banned flag
    if req.ban_type == "account" {
        let _ = sqlx::query("UPDATE accounts SET is_banned = 1, ban_reason = ? WHERE username = ?")
            .bind(&req.reason)
            .bind(&req.value.to_lowercase())
            .execute(&state.db)
            .await;
    }

    Ok(Json(ApiResponse::success(CreateBanResponse {
        id: result.last_insert_rowid(),
    })))
}

#[derive(Serialize)]
pub struct DeleteBanResponse {
    pub deleted: bool,
}

/// DELETE /api/bans/:id - Remove a ban
pub async fn delete_ban(
    headers: HeaderMap,
    Path(id): Path<i64>,
    State(state): State<Arc<AdminState>>,
) -> Result<Json<ApiResponse<DeleteBanResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    // Get ban info first (to update account if needed)
    let ban_info: Option<(String, String)> = sqlx::query_as(
        "SELECT ban_type, value FROM bans WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("Database error: {}", e))),
        )
    })?;

    let result = sqlx::query("DELETE FROM bans WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!("Failed to delete ban: {}", e))),
            )
        })?;

    // If it was an account ban, also clear the account's is_banned flag
    if let Some((ban_type, value)) = ban_info {
        if ban_type == "account" {
            let _ = sqlx::query("UPDATE accounts SET is_banned = 0, ban_reason = NULL WHERE username = ?")
                .bind(&value.to_lowercase())
                .execute(&state.db)
                .await;
        }
    }

    Ok(Json(ApiResponse::success(DeleteBanResponse {
        deleted: result.rows_affected() > 0,
    })))
}
