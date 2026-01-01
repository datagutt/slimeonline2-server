//! Admin HTTP API for remote server management
//!
//! Provides REST endpoints for moderator tools to interact with the server.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::db::DbPool;
use crate::protocol::MessageWriter;

mod handlers;

/// Admin action that needs to be executed by the main game loop
#[derive(Debug, Clone)]
pub enum AdminAction {
    /// Kick a player by username
    KickPlayer {
        username: String,
        reason: Option<String>,
    },
    /// Teleport a player to a specific location
    TeleportPlayer {
        username: String,
        room_id: u16,
        x: u16,
        y: u16,
    },
    /// Give/set points for a player (updates DB + sends MSG_POINTS to client)
    SetPoints {
        username: String,
        points: i64,
        mode: PointsMode,
    },
    /// Give an item to a player's inventory slot (updates DB + notifies client)
    GiveItem {
        username: String,
        category: InventoryCategory,
        slot: u8,
        item_id: u16,
    },
    /// Set bank balance
    SetBank {
        username: String,
        balance: i64,
        mode: PointsMode,
    },
    /// Send system mail to a player (creates mail + sends notification if online)
    SendMail {
        to_username: String,
        sender_name: String,
        message: String,
        points: i64,
        item_id: u16,
        item_category: u8,
    },
    /// Change player appearance (body/accessories)
    SetAppearance {
        username: String,
        body_id: Option<u16>,
        acs1_id: Option<u16>,
        acs2_id: Option<u16>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointsMode {
    Set,
    Add,
    Subtract,
}

impl PointsMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "set" => Some(Self::Set),
            "add" => Some(Self::Add),
            "subtract" | "sub" => Some(Self::Subtract),
            _ => None,
        }
    }

    pub fn apply(&self, current: i64, value: i64) -> i64 {
        match self {
            Self::Set => value,
            Self::Add => current.saturating_add(value),
            Self::Subtract => current.saturating_sub(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryCategory {
    Item,
    Outfit,
    Accessory,
    Tool,
    Emote,
}

impl InventoryCategory {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "item" | "items" => Some(Self::Item),
            "outfit" | "outfits" => Some(Self::Outfit),
            "accessory" | "accessories" | "acs" => Some(Self::Accessory),
            "tool" | "tools" => Some(Self::Tool),
            "emote" | "emotes" => Some(Self::Emote),
            _ => None,
        }
    }

    pub fn max_slot(&self) -> u8 {
        match self {
            Self::Emote => 5,
            _ => 9,
        }
    }
}

/// Shared state for admin API handlers
pub struct AdminState {
    pub db: DbPool,
    pub api_key: String,
    /// Channel to send actions to the main game loop
    pub action_tx: mpsc::Sender<AdminAction>,
    /// Reference to server sessions for real-time queries
    pub sessions: Arc<dashmap::DashMap<uuid::Uuid, Arc<tokio::sync::RwLock<crate::game::PlayerSession>>>>,
    /// Reference to game state for room info
    pub game_state: Arc<crate::game::GameState>,
}

/// Standard API response wrapper
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Verify the API key from request headers
fn verify_api_key(headers: &HeaderMap, expected_key: &str) -> Result<(), (StatusCode, Json<ApiResponse<()>>)> {
    let provided_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .or_else(|| {
            headers
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
        });

    match provided_key {
        Some(key) if key == expected_key => Ok(()),
        Some(_) => Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Invalid API key")),
        )),
        None => Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Missing API key. Use X-API-Key header or Authorization: Bearer <key>")),
        )),
    }
}

/// Create the admin API router
pub fn create_router(state: Arc<AdminState>) -> Router {
    Router::new()
        // Server endpoints
        .route("/api/server/stats", get(handlers::server::get_stats))
        // Player endpoints
        .route("/api/players", get(handlers::players::list_online))
        .route("/api/players/:username", get(handlers::players::get_info))
        .route("/api/players/:username/kick", post(handlers::players::kick))
        .route("/api/players/:username/ban", post(handlers::players::ban))
        .route("/api/players/:username/teleport", post(handlers::players::teleport))
        .route("/api/players/:username/points", post(handlers::players::set_points))
        .route("/api/players/:username/bank", post(handlers::players::set_bank))
        .route("/api/players/:username/inventory", post(handlers::players::set_inventory_slot))
        .route("/api/players/:username/moderator", post(handlers::players::set_moderator))
        // Ban management
        .route("/api/bans", get(handlers::bans::list_bans))
        .route("/api/bans", post(handlers::bans::create_ban))
        .route("/api/bans/:id", delete(handlers::bans::delete_ban))
        // Mail endpoints
        .route("/api/mail/send", post(handlers::mail::send_system_mail))
        .route("/api/mail/:username", get(handlers::mail::get_mailbox))
        // Clan endpoints
        .route("/api/clans", get(handlers::clans::list_clans))
        .route("/api/clans/:name", get(handlers::clans::get_clan))
        .route("/api/clans/:name", delete(handlers::clans::dissolve_clan))
        .route("/api/clans/:name/points", post(handlers::clans::add_points))
        // Account endpoints
        .route("/api/accounts", get(handlers::accounts::list_accounts))
        .route("/api/accounts/:username", get(handlers::accounts::get_account))
        .with_state(state)
}

/// Start the admin API server
pub async fn start_server(
    host: &str,
    port: u16,
    state: Arc<AdminState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_router(state);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("Admin API listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
