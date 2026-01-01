//! Clan admin endpoints

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::admin::{verify_api_key, AdminState, ApiResponse};
use crate::db;

#[derive(Serialize)]
pub struct ClanSummary {
    pub id: i64,
    pub name: String,
    pub leader_id: i64,
    pub member_count: i64,
    pub max_members: i64,
    pub points: i64,
    pub level: i64,
}

/// GET /api/clans - List all clans
pub async fn list_clans(
    headers: HeaderMap,
    State(state): State<Arc<AdminState>>,
) -> Result<Json<ApiResponse<Vec<ClanSummary>>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let clans: Vec<(i64, String, i64, i64, i64, i64)> = sqlx::query_as(
        r#"
        SELECT c.id, c.name, c.leader_id, c.max_members, c.points, c.level
        FROM clans c
        ORDER BY c.points DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Database error: {}", e))),
        )
    })?;

    let mut result = Vec::new();
    for (id, name, leader_id, max_members, points, level) in clans {
        let member_count = db::get_clan_member_count(&state.db, id).await.unwrap_or(0);
        result.push(ClanSummary {
            id,
            name,
            leader_id,
            member_count,
            max_members,
            points,
            level,
        });
    }

    Ok(Json(ApiResponse::success(result)))
}

#[derive(Serialize)]
pub struct ClanMemberInfo {
    pub character_id: i64,
    pub username: String,
    pub is_leader: bool,
}

#[derive(Serialize)]
pub struct ClanDetail {
    pub id: i64,
    pub name: String,
    pub leader_id: i64,
    pub color_inner: i64,
    pub color_outer: i64,
    pub level: i64,
    pub points: i64,
    pub max_members: i64,
    pub description: Option<String>,
    pub news: Option<String>,
    pub show_name: bool,
    pub has_base: bool,
    pub created_at: String,
    pub members: Vec<ClanMemberInfo>,
}

/// GET /api/clans/:name - Get detailed clan info
pub async fn get_clan(
    headers: HeaderMap,
    Path(name): Path<String>,
    State(state): State<Arc<AdminState>>,
) -> Result<Json<ApiResponse<ClanDetail>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let clan = match db::get_clan_by_name(&state.db, &name).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!("Clan '{}' not found", name))),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Database error: {}", e))),
            ))
        }
    };

    let members_raw = db::get_clan_members(&state.db, clan.id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Database error: {}", e))),
        )
    })?;

    let members: Vec<ClanMemberInfo> = members_raw
        .into_iter()
        .map(|m| ClanMemberInfo {
            character_id: m.character_id,
            username: m.username,
            is_leader: m.character_id == clan.leader_id,
        })
        .collect();

    Ok(Json(ApiResponse::success(ClanDetail {
        id: clan.id,
        name: clan.name,
        leader_id: clan.leader_id,
        color_inner: clan.color_inner,
        color_outer: clan.color_outer,
        level: clan.level,
        points: clan.points,
        max_members: clan.max_members,
        description: clan.description,
        news: clan.news,
        show_name: clan.show_name != 0,
        has_base: clan.has_base != 0,
        created_at: clan.created_at,
        members,
    })))
}

#[derive(Serialize)]
pub struct DissolveClanResponse {
    pub dissolved: bool,
    pub members_removed: i64,
}

/// DELETE /api/clans/:name - Dissolve a clan
pub async fn dissolve_clan(
    headers: HeaderMap,
    Path(name): Path<String>,
    State(state): State<Arc<AdminState>>,
) -> Result<Json<ApiResponse<DissolveClanResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let clan = match db::get_clan_by_name(&state.db, &name).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!("Clan '{}' not found", name))),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Database error: {}", e))),
            ))
        }
    };

    let member_count = db::get_clan_member_count(&state.db, clan.id).await.unwrap_or(0);

    if let Err(e) = db::dissolve_clan(&state.db, clan.id).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Failed to dissolve clan: {}", e))),
        ));
    }

    // TODO: Notify online clan members via AdminAction

    Ok(Json(ApiResponse::success(DissolveClanResponse {
        dissolved: true,
        members_removed: member_count,
    })))
}

#[derive(Deserialize)]
pub struct AddPointsRequest {
    pub points: i64,
}

#[derive(Serialize)]
pub struct AddPointsResponse {
    pub new_total: i64,
}

/// POST /api/clans/:name/points - Add points to a clan
pub async fn add_points(
    headers: HeaderMap,
    Path(name): Path<String>,
    State(state): State<Arc<AdminState>>,
    Json(req): Json<AddPointsRequest>,
) -> Result<Json<ApiResponse<AddPointsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    verify_api_key(&headers, &state.api_key)?;

    let clan = match db::get_clan_by_name(&state.db, &name).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(format!("Clan '{}' not found", name))),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Database error: {}", e))),
            ))
        }
    };

    if let Err(e) = db::add_clan_points(&state.db, clan.id, req.points).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("Failed to add points: {}", e))),
        ));
    }

    // Get updated clan to return new total
    let new_total = match db::get_clan(&state.db, clan.id).await {
        Ok(Some(c)) => c.points,
        _ => clan.points + req.points,
    };

    Ok(Json(ApiResponse::success(AddPointsResponse { new_total })))
}
