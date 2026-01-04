//! Server-wide admin endpoints

use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Serialize;

use crate::admin::{AdminState, ApiResponse, verify_api_key};

#[derive(Serialize)]
pub struct ServerStats {
    pub online_players: usize,
    pub total_connections: usize,
    pub rooms_active: usize,
}

/// GET /api/server/stats - Get server statistics
pub async fn get_stats(
    headers: HeaderMap,
    State(state): State<Arc<AdminState>>,
) -> Result<Json<ApiResponse<ServerStats>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

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

    let stats = ServerStats {
        online_players,
        total_connections: state.sessions.len(),
        rooms_active: state.game_state.rooms.len(),
    };

    Ok(Json(ApiResponse::success(stats)))
}
