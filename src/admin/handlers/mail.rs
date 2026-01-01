//! Mail admin endpoints

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::admin::{verify_api_key, AdminAction, AdminState, ApiResponse};
use crate::db;

#[derive(Deserialize)]
pub struct SendMailRequest {
    pub to: String,
    pub message: String,
    #[serde(default = "default_sender")]
    pub sender: String,
    #[serde(default)]
    pub points: i64,
    #[serde(default)]
    pub item_id: u16,
    #[serde(default)]
    pub item_category: u8, // 1=outfit, 2=item, 3=accessory, 4=tool
    #[serde(default = "default_paper")]
    pub paper: u8,
    #[serde(default)]
    pub font_color: u8,
}

fn default_sender() -> String {
    "System".to_string()
}

fn default_paper() -> u8 {
    1
}

#[derive(Serialize)]
pub struct SendMailResponse {
    pub queued: bool,
}

/// POST /api/mail/send - Send system mail to a player
pub async fn send_system_mail(
    headers: HeaderMap,
    State(state): State<Arc<AdminState>>,
    Json(req): Json<SendMailRequest>,
) -> Result<Json<ApiResponse<SendMailResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    if req.to.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Recipient cannot be empty")),
        ));
    }

    if req.message.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Message cannot be empty")),
        ));
    }

    if req.message.len() > 1000 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Message too long (max 1000 chars)")),
        ));
    }

    // Validate item category if item is provided
    if req.item_id > 0 && req.item_category == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("item_category required when item_id is provided (1=outfit, 2=item, 3=accessory, 4=tool)")),
        ));
    }

    if state
        .action_tx
        .send(AdminAction::SendMail {
            to_username: req.to.to_lowercase(),
            sender_name: req.sender,
            message: req.message,
            points: req.points,
            item_id: req.item_id,
            item_category: req.item_category,
        })
        .await
        .is_err()
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("Failed to queue mail action")),
        ));
    }

    Ok(Json(ApiResponse::success(SendMailResponse { queued: true })))
}

#[derive(Serialize)]
pub struct MailEntry {
    pub id: i64,
    pub from: String,
    pub message: String,
    pub item_id: i64,
    pub item_category: i64,
    pub points: i64,
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct MailboxResponse {
    pub total: i64,
    pub unread: i64,
    pub mail: Vec<MailEntry>,
}

#[derive(Deserialize, Default)]
pub struct MailboxQuery {
    pub page: Option<i64>,
}

/// GET /api/mail/:username - Get a player's mailbox
pub async fn get_mailbox(
    headers: HeaderMap,
    Path(username): Path<String>,
    Query(query): Query<MailboxQuery>,
    State(state): State<Arc<AdminState>>,
) -> Result<Json<ApiResponse<MailboxResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let username_lower = username.to_lowercase();
    let page = query.page.unwrap_or(0);

    // Get character ID
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

    // Get mail
    let mail_list = db::get_mailbox(&state.db, character.id, page)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Database error: {}", e))),
            )
        })?;

    let total = db::get_mail_count(&state.db, character.id)
        .await
        .unwrap_or(0);

    let unread = db::get_unread_mail_count(&state.db, character.id)
        .await
        .unwrap_or(0);

    let mail: Vec<MailEntry> = mail_list
        .into_iter()
        .map(|m| MailEntry {
            id: m.id,
            from: m.sender_name,
            message: m.message,
            item_id: m.item_id,
            item_category: m.item_cat,
            points: m.points,
            is_read: m.is_read != 0,
            created_at: m.created_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(MailboxResponse {
        total,
        unread,
        mail,
    })))
}
