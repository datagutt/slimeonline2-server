//! Clan database operations

use super::DbPool;
use tracing::debug;

/// Clan data from the database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Clan {
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
    pub show_name: i64,
    pub has_base: i64,
    pub created_at: String,
}

/// Clan member info
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ClanMember {
    pub character_id: i64,
    pub username: String,
}

/// Create a new clan
pub async fn create_clan(
    pool: &DbPool,
    name: &str,
    leader_id: i64,
    initial_slots: u8,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO clans (name, leader_id, max_members, description, news)
        VALUES (?, ?, ?, 'A new clan', 'No news')
        "#,
    )
    .bind(name)
    .bind(leader_id)
    .bind(initial_slots as i64)
    .execute(pool)
    .await?;

    let clan_id = result.last_insert_rowid();

    // Update the leader's clan_id
    sqlx::query("UPDATE characters SET clan_id = ? WHERE id = ?")
        .bind(clan_id)
        .bind(leader_id)
        .execute(pool)
        .await?;

    debug!("Created clan '{}' (id={}) with leader_id={}", name, clan_id, leader_id);
    Ok(clan_id)
}

/// Get a clan by ID
pub async fn get_clan(pool: &DbPool, clan_id: i64) -> Result<Option<Clan>, sqlx::Error> {
    sqlx::query_as::<_, Clan>("SELECT * FROM clans WHERE id = ?")
        .bind(clan_id)
        .fetch_optional(pool)
        .await
}

/// Get a clan by name (case-insensitive)
pub async fn get_clan_by_name(pool: &DbPool, name: &str) -> Result<Option<Clan>, sqlx::Error> {
    sqlx::query_as::<_, Clan>("SELECT * FROM clans WHERE LOWER(name) = LOWER(?)")
        .bind(name)
        .fetch_optional(pool)
        .await
}

/// Check if a clan name is already taken
pub async fn is_clan_name_taken(pool: &DbPool, name: &str) -> Result<bool, sqlx::Error> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM clans WHERE LOWER(name) = LOWER(?)")
        .bind(name)
        .fetch_one(pool)
        .await?;
    Ok(count.0 > 0)
}

/// Get all members of a clan (including leader)
pub async fn get_clan_members(pool: &DbPool, clan_id: i64) -> Result<Vec<ClanMember>, sqlx::Error> {
    sqlx::query_as::<_, ClanMember>(
        r#"
        SELECT id as character_id, username 
        FROM characters 
        WHERE clan_id = ?
        ORDER BY id
        "#,
    )
    .bind(clan_id)
    .fetch_all(pool)
    .await
}

/// Get the number of members in a clan
pub async fn get_clan_member_count(pool: &DbPool, clan_id: i64) -> Result<i64, sqlx::Error> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM characters WHERE clan_id = ?")
        .bind(clan_id)
        .fetch_one(pool)
        .await?;
    Ok(count.0)
}

/// Add a character to a clan
pub async fn add_clan_member(
    pool: &DbPool,
    clan_id: i64,
    character_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE characters SET clan_id = ? WHERE id = ?")
        .bind(clan_id)
        .bind(character_id)
        .execute(pool)
        .await?;
    debug!("Added character {} to clan {}", character_id, clan_id);
    Ok(())
}

/// Remove a character from their clan
pub async fn remove_clan_member(pool: &DbPool, character_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE characters SET clan_id = NULL WHERE id = ?")
        .bind(character_id)
        .execute(pool)
        .await?;
    debug!("Removed character {} from their clan", character_id);
    Ok(())
}

/// Delete a clan and remove all members
pub async fn dissolve_clan(pool: &DbPool, clan_id: i64) -> Result<(), sqlx::Error> {
    // First remove all members from the clan
    sqlx::query("UPDATE characters SET clan_id = NULL WHERE clan_id = ?")
        .bind(clan_id)
        .execute(pool)
        .await?;

    // Then delete the clan
    sqlx::query("DELETE FROM clans WHERE id = ?")
        .bind(clan_id)
        .execute(pool)
        .await?;

    debug!("Dissolved clan {}", clan_id);
    Ok(())
}

/// Update clan colors
pub async fn update_clan_colors(
    pool: &DbPool,
    clan_id: i64,
    inner_color: u32,
    outer_color: u32,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE clans SET color_inner = ?, color_outer = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(inner_color as i64)
        .bind(outer_color as i64)
        .bind(clan_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update clan info text
pub async fn update_clan_info(
    pool: &DbPool,
    clan_id: i64,
    show_leader: bool,
    description: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE clans SET show_name = ?, description = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(if show_leader { 1 } else { 0 })
        .bind(description)
        .bind(clan_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update clan news
pub async fn update_clan_news(pool: &DbPool, clan_id: i64, news: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE clans SET news = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(news)
        .bind(clan_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Add points to a clan
pub async fn add_clan_points(pool: &DbPool, clan_id: i64, points: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE clans SET points = points + ?, updated_at = datetime('now') WHERE id = ?")
        .bind(points)
        .bind(clan_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get a character's clan_id
pub async fn get_character_clan_id(pool: &DbPool, character_id: i64) -> Result<Option<i64>, sqlx::Error> {
    let result: Option<(Option<i64>,)> = sqlx::query_as(
        "SELECT clan_id FROM characters WHERE id = ?"
    )
    .bind(character_id)
    .fetch_optional(pool)
    .await?;
    
    Ok(result.and_then(|r| r.0))
}

/// Check if a character is the leader of their clan
pub async fn is_clan_leader(pool: &DbPool, character_id: i64, clan_id: i64) -> Result<bool, sqlx::Error> {
    let result: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM clans WHERE id = ? AND leader_id = ?"
    )
    .bind(clan_id)
    .bind(character_id)
    .fetch_optional(pool)
    .await?;
    
    Ok(result.is_some())
}

/// Increase max member slots for a clan
pub async fn increase_clan_slots(pool: &DbPool, clan_id: i64) -> Result<i64, sqlx::Error> {
    sqlx::query("UPDATE clans SET max_members = max_members + 1, updated_at = datetime('now') WHERE id = ?")
        .bind(clan_id)
        .execute(pool)
        .await?;
    
    let result: (i64,) = sqlx::query_as("SELECT max_members FROM clans WHERE id = ?")
        .bind(clan_id)
        .fetch_one(pool)
        .await?;
    
    Ok(result.0)
}
