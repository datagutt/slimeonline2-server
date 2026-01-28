//! Web server for landing page and public API
//!
//! Serves static files and provides public endpoints that don't require API authentication.

use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use tower_http::services::ServeDir;
use tracing::info;

use crate::game::{GameState, SessionHandle};

/// Shared state for web handlers
pub struct WebState {
    /// Reference to server sessions for real-time stats
    pub sessions: Arc<dashmap::DashMap<uuid::Uuid, Arc<SessionHandle>>>,
    /// Reference to game state for room info
    pub game_state: Arc<GameState>,
}

/// Public server statistics (no auth required)
#[derive(Serialize)]
pub struct PublicStats {
    pub online_players: usize,
    pub rooms_active: usize,
}

/// GET /api/stats - Get public server statistics (no auth required)
async fn get_public_stats(State(state): State<Arc<WebState>>) -> Json<PublicStats> {
    let online_players = state
        .sessions
        .iter()
        .filter(|s| {
            s.value()
                .session
                .try_read()
                .map(|s| s.is_authenticated)
                .unwrap_or(false)
        })
        .count();

    Json(PublicStats {
        online_players,
        rooms_active: state.game_state.rooms.len(),
    })
}

/// Create the web router with static file serving and public API
pub fn create_router(state: Arc<WebState>) -> Router {
    Router::new()
        // Public API endpoint (no auth required)
        .route("/api/stats", get(get_public_stats))
        .with_state(state)
        // Serve static files from web/ directory
        .nest_service("/", ServeDir::new("web"))
}

/// Start the web server
pub async fn start_server(
    host: &str,
    port: u16,
    state: Arc<WebState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_router(state);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("Web server listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
