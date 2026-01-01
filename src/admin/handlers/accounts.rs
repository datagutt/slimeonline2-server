//! Account admin endpoints

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::admin::{verify_api_key, AdminState, ApiResponse};
use crate::db;

#[derive(Serialize)]
pub struct AccountSummary {
    pub id: i64,
    pub username: String,
    pub is_banned: bool,
    pub created_at: String,
    pub last_login: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct ListAccountsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
}

/// GET /api/accounts - List accounts
pub async fn list_accounts(
    headers: HeaderMap,
    State(state): State<Arc<AdminState>>,
    Query(query): Query<ListAccountsQuery>,
) -> Result<Json<ApiResponse<Vec<AccountSummary>>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);

    let accounts: Vec<AccountSummary> = if let Some(search) = &query.search {
        let pattern = format!("%{}%", search.to_lowercase());
        sqlx::query_as::<_, (i64, String, bool, String, Option<String>)>(
            r#"
            SELECT id, username, is_banned, created_at, last_login
            FROM accounts
            WHERE username LIKE ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!("Database error: {}", e))),
            )
        })?
        .into_iter()
        .map(|(id, username, is_banned, created_at, last_login)| AccountSummary {
            id,
            username,
            is_banned,
            created_at,
            last_login,
        })
        .collect()
    } else {
        sqlx::query_as::<_, (i64, String, bool, String, Option<String>)>(
            r#"
            SELECT id, username, is_banned, created_at, last_login
            FROM accounts
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!("Database error: {}", e))),
            )
        })?
        .into_iter()
        .map(|(id, username, is_banned, created_at, last_login)| AccountSummary {
            id,
            username,
            is_banned,
            created_at,
            last_login,
        })
        .collect()
    };

    Ok(Json(ApiResponse::success(accounts)))
}

#[derive(Serialize)]
pub struct AccountDetail {
    pub id: i64,
    pub username: String,
    pub mac_address: String,
    pub is_banned: bool,
    pub ban_reason: Option<String>,
    pub created_at: String,
    pub last_login: Option<String>,
    pub has_character: bool,
    pub character_id: Option<i64>,
    pub is_online: bool,
}

/// GET /api/accounts/:username - Get account details
pub async fn get_account(
    headers: HeaderMap,
    Path(username): Path<String>,
    State(state): State<Arc<AdminState>>,
) -> Result<Json<ApiResponse<AccountDetail>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let username_lower = username.to_lowercase();

    let account = match db::find_account_by_username(&state.db, &username_lower).await {
        Ok(Some(acc)) => acc,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error(format!("Account '{}' not found", username))),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!("Database error: {}", e))),
            ))
        }
    };

    let character = db::find_character_by_account(&state.db, account.id)
        .await
        .ok()
        .flatten();

    let is_online = state.sessions.iter().any(|s| {
        s.value()
            .try_read()
            .map(|s| s.username.as_deref() == Some(&username_lower) && s.is_authenticated)
            .unwrap_or(false)
    });

    Ok(Json(ApiResponse::success(AccountDetail {
        id: account.id,
        username: account.username,
        mac_address: account.mac_address,
        is_banned: account.is_banned,
        ban_reason: account.ban_reason,
        created_at: account.created_at,
        last_login: account.last_login,
        has_character: character.is_some(),
        character_id: character.map(|c| c.id),
        is_online,
    })))
}
