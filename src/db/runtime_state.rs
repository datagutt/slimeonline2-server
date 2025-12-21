//! Runtime state database operations
//!
//! Handles persistence of collectible state, plant state, shop stock,
//! and ground items across server restarts.

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use super::DbPool;

// =============================================================================
// Collectible State
// =============================================================================

#[derive(Debug, Clone, FromRow)]
pub struct CollectibleState {
    pub room_id: i64,
    pub spawn_id: i64,
    pub available: i64,
    pub respawn_at: Option<String>,
    pub current_item_id: Option<i64>,
}

/// Get collectible state for a room
pub async fn get_collectible_states(
    pool: &DbPool,
    room_id: u16,
) -> Result<Vec<CollectibleState>, sqlx::Error> {
    sqlx::query_as::<_, CollectibleState>(
        r#"
        SELECT room_id, spawn_id, available, respawn_at, current_item_id
        FROM collectible_state
        WHERE room_id = ?
        "#,
    )
    .bind(room_id as i64)
    .fetch_all(pool)
    .await
}

/// Get a single collectible's state
pub async fn get_collectible_state(
    pool: &DbPool,
    room_id: u16,
    spawn_id: u8,
) -> Result<Option<CollectibleState>, sqlx::Error> {
    sqlx::query_as::<_, CollectibleState>(
        r#"
        SELECT room_id, spawn_id, available, respawn_at, current_item_id
        FROM collectible_state
        WHERE room_id = ? AND spawn_id = ?
        "#,
    )
    .bind(room_id as i64)
    .bind(spawn_id as i64)
    .fetch_optional(pool)
    .await
}

/// Mark a collectible as taken and set respawn time
pub async fn take_collectible(
    pool: &DbPool,
    room_id: u16,
    spawn_id: u8,
    respawn_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO collectible_state (room_id, spawn_id, available, respawn_at)
        VALUES (?, ?, 0, ?)
        ON CONFLICT (room_id, spawn_id) DO UPDATE SET
            available = 0,
            respawn_at = excluded.respawn_at
        "#,
    )
    .bind(room_id as i64)
    .bind(spawn_id as i64)
    .bind(respawn_at.to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

/// Mark a collectible as available again
pub async fn respawn_collectible(
    pool: &DbPool,
    room_id: u16,
    spawn_id: u8,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE collectible_state
        SET available = 1, respawn_at = NULL, current_item_id = NULL
        WHERE room_id = ? AND spawn_id = ?
        "#,
    )
    .bind(room_id as i64)
    .bind(spawn_id as i64)
    .execute(pool)
    .await?;
    Ok(())
}

/// Update a collectible's current item (for evolution)
pub async fn update_collectible_item(
    pool: &DbPool,
    room_id: u16,
    spawn_id: u8,
    item_id: u16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO collectible_state (room_id, spawn_id, available, current_item_id)
        VALUES (?, ?, 1, ?)
        ON CONFLICT (room_id, spawn_id) DO UPDATE SET
            current_item_id = excluded.current_item_id
        "#,
    )
    .bind(room_id as i64)
    .bind(spawn_id as i64)
    .bind(item_id as i64)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get all collectibles that need to respawn (respawn_at <= now)
pub async fn get_collectibles_to_respawn(
    pool: &DbPool,
) -> Result<Vec<CollectibleState>, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query_as::<_, CollectibleState>(
        r#"
        SELECT room_id, spawn_id, available, respawn_at, current_item_id
        FROM collectible_state
        WHERE available = 0 AND respawn_at IS NOT NULL AND respawn_at <= ?
        "#,
    )
    .bind(now)
    .fetch_all(pool)
    .await
}

// =============================================================================
// Plant State
// =============================================================================

#[derive(Debug, Clone, FromRow)]
pub struct PlantState {
    pub room_id: i64,
    pub spot_id: i64,
    pub owner_id: Option<i64>,
    pub seed_id: Option<i64>,
    pub stage: i64,
    pub fairy_count: i64,
    pub pinwheel_id: Option<i64>,
    pub planted_at: Option<String>,
    pub next_stage_at: Option<String>,
    pub has_fruit: i64,
}

/// Get plant state for a room
pub async fn get_plant_states(
    pool: &DbPool,
    room_id: u16,
) -> Result<Vec<PlantState>, sqlx::Error> {
    sqlx::query_as::<_, PlantState>(
        r#"
        SELECT room_id, spot_id, owner_id, seed_id, stage, fairy_count,
               pinwheel_id, planted_at, next_stage_at, has_fruit
        FROM plant_state
        WHERE room_id = ?
        "#,
    )
    .bind(room_id as i64)
    .fetch_all(pool)
    .await
}

/// Get a single plant's state
pub async fn get_plant_state(
    pool: &DbPool,
    room_id: u16,
    spot_id: u8,
) -> Result<Option<PlantState>, sqlx::Error> {
    sqlx::query_as::<_, PlantState>(
        r#"
        SELECT room_id, spot_id, owner_id, seed_id, stage, fairy_count,
               pinwheel_id, planted_at, next_stage_at, has_fruit
        FROM plant_state
        WHERE room_id = ? AND spot_id = ?
        "#,
    )
    .bind(room_id as i64)
    .bind(spot_id as i64)
    .fetch_optional(pool)
    .await
}

/// Plant a seed
pub async fn plant_seed(
    pool: &DbPool,
    room_id: u16,
    spot_id: u8,
    owner_id: i64,
    seed_id: u16,
    next_stage_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO plant_state (room_id, spot_id, owner_id, seed_id, stage, planted_at, next_stage_at)
        VALUES (?, ?, ?, ?, 0, ?, ?)
        ON CONFLICT (room_id, spot_id) DO UPDATE SET
            owner_id = excluded.owner_id,
            seed_id = excluded.seed_id,
            stage = 0,
            fairy_count = 0,
            pinwheel_id = NULL,
            planted_at = excluded.planted_at,
            next_stage_at = excluded.next_stage_at,
            has_fruit = 0
        "#,
    )
    .bind(room_id as i64)
    .bind(spot_id as i64)
    .bind(owner_id)
    .bind(seed_id as i64)
    .bind(&now)
    .bind(next_stage_at.to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

/// Advance plant to next stage
pub async fn advance_plant_stage(
    pool: &DbPool,
    room_id: u16,
    spot_id: u8,
    new_stage: u8,
    next_stage_at: Option<DateTime<Utc>>,
    has_fruit: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE plant_state
        SET stage = ?, next_stage_at = ?, has_fruit = ?
        WHERE room_id = ? AND spot_id = ?
        "#,
    )
    .bind(new_stage as i64)
    .bind(next_stage_at.map(|t| t.to_rfc3339()))
    .bind(if has_fruit { 1i64 } else { 0i64 })
    .bind(room_id as i64)
    .bind(spot_id as i64)
    .execute(pool)
    .await?;
    Ok(())
}

/// Add a fairy to a plant
pub async fn add_fairy_to_plant(
    pool: &DbPool,
    room_id: u16,
    spot_id: u8,
) -> Result<u8, sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE plant_state
        SET fairy_count = MIN(fairy_count + 1, 5)
        WHERE room_id = ? AND spot_id = ?
        "#,
    )
    .bind(room_id as i64)
    .bind(spot_id as i64)
    .execute(pool)
    .await?;
    
    // Return new fairy count
    let state = get_plant_state(pool, room_id, spot_id).await?;
    Ok(state.map(|s| s.fairy_count as u8).unwrap_or(0))
}

/// Add a pinwheel to a plant
pub async fn add_pinwheel_to_plant(
    pool: &DbPool,
    room_id: u16,
    spot_id: u8,
    pinwheel_id: u16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE plant_state
        SET pinwheel_id = ?
        WHERE room_id = ? AND spot_id = ?
        "#,
    )
    .bind(pinwheel_id as i64)
    .bind(room_id as i64)
    .bind(spot_id as i64)
    .execute(pool)
    .await?;
    Ok(())
}

/// Harvest fruit from plant (resets has_fruit)
pub async fn harvest_plant(
    pool: &DbPool,
    room_id: u16,
    spot_id: u8,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE plant_state
        SET has_fruit = 0
        WHERE room_id = ? AND spot_id = ?
        "#,
    )
    .bind(room_id as i64)
    .bind(spot_id as i64)
    .execute(pool)
    .await?;
    Ok(())
}

/// Clear a plant spot (remove plant)
pub async fn clear_plant(
    pool: &DbPool,
    room_id: u16,
    spot_id: u8,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM plant_state
        WHERE room_id = ? AND spot_id = ?
        "#,
    )
    .bind(room_id as i64)
    .bind(spot_id as i64)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get all plants that need to advance stage
pub async fn get_plants_to_advance(pool: &DbPool) -> Result<Vec<PlantState>, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query_as::<_, PlantState>(
        r#"
        SELECT room_id, spot_id, owner_id, seed_id, stage, fairy_count,
               pinwheel_id, planted_at, next_stage_at, has_fruit
        FROM plant_state
        WHERE stage < 6 AND next_stage_at IS NOT NULL AND next_stage_at <= ?
        "#,
    )
    .bind(now)
    .fetch_all(pool)
    .await
}

// =============================================================================
// Shop Stock
// =============================================================================

#[derive(Debug, Clone, FromRow)]
pub struct ShopStock {
    pub room_id: i64,
    pub slot_id: i64,
    pub current_stock: i64,
    pub last_purchase: Option<String>,
    pub last_restock: Option<String>,
}

/// Get shop stock for a room
pub async fn get_shop_stock(
    pool: &DbPool,
    room_id: u16,
) -> Result<Vec<ShopStock>, sqlx::Error> {
    sqlx::query_as::<_, ShopStock>(
        r#"
        SELECT room_id, slot_id, current_stock, last_purchase, last_restock
        FROM shop_stock
        WHERE room_id = ?
        "#,
    )
    .bind(room_id as i64)
    .fetch_all(pool)
    .await
}

/// Get stock for a specific shop slot
pub async fn get_shop_slot_stock(
    pool: &DbPool,
    room_id: u16,
    slot_id: u8,
) -> Result<Option<ShopStock>, sqlx::Error> {
    sqlx::query_as::<_, ShopStock>(
        r#"
        SELECT room_id, slot_id, current_stock, last_purchase, last_restock
        FROM shop_stock
        WHERE room_id = ? AND slot_id = ?
        "#,
    )
    .bind(room_id as i64)
    .bind(slot_id as i64)
    .fetch_optional(pool)
    .await
}

/// Decrease shop stock after purchase
pub async fn decrease_shop_stock(
    pool: &DbPool,
    room_id: u16,
    slot_id: u8,
    initial_stock: u16,
) -> Result<u16, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    
    // Insert or update stock
    sqlx::query(
        r#"
        INSERT INTO shop_stock (room_id, slot_id, current_stock, last_purchase)
        VALUES (?, ?, ?, ?)
        ON CONFLICT (room_id, slot_id) DO UPDATE SET
            current_stock = MAX(shop_stock.current_stock - 1, 0),
            last_purchase = excluded.last_purchase
        "#,
    )
    .bind(room_id as i64)
    .bind(slot_id as i64)
    .bind((initial_stock.saturating_sub(1)) as i64)
    .bind(&now)
    .execute(pool)
    .await?;
    
    // Return new stock
    let stock = get_shop_slot_stock(pool, room_id, slot_id).await?;
    Ok(stock.map(|s| s.current_stock as u16).unwrap_or(0))
}

/// Restock a shop slot to its max value
pub async fn restock_shop_slot(
    pool: &DbPool,
    room_id: u16,
    slot_id: u8,
    max_stock: u16,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO shop_stock (room_id, slot_id, current_stock, last_restock)
        VALUES (?, ?, ?, ?)
        ON CONFLICT (room_id, slot_id) DO UPDATE SET
            current_stock = excluded.current_stock,
            last_restock = excluded.last_restock
        "#,
    )
    .bind(room_id as i64)
    .bind(slot_id as i64)
    .bind(max_stock as i64)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

// =============================================================================
// Ground Items (Discarded)
// =============================================================================

#[derive(Debug, Clone, FromRow)]
pub struct GroundItem {
    pub id: i64,
    pub room_id: i64,
    pub item_id: i64,
    pub x: i64,
    pub y: i64,
    pub dropped_by: Option<i64>,
    pub dropped_at: String,
    pub expires_at: Option<String>,
}

/// Get ground items in a room
pub async fn get_ground_items(
    pool: &DbPool,
    room_id: u16,
) -> Result<Vec<GroundItem>, sqlx::Error> {
    sqlx::query_as::<_, GroundItem>(
        r#"
        SELECT id, room_id, item_id, x, y, dropped_by, dropped_at, expires_at
        FROM ground_items
        WHERE room_id = ?
        ORDER BY id ASC
        "#,
    )
    .bind(room_id as i64)
    .fetch_all(pool)
    .await
}

/// Add a ground item
pub async fn add_ground_item(
    pool: &DbPool,
    room_id: u16,
    item_id: u16,
    x: u16,
    y: u16,
    dropped_by: Option<i64>,
    expires_in_secs: Option<u32>,
) -> Result<i64, sqlx::Error> {
    let now = Utc::now();
    let dropped_at = now.to_rfc3339();
    let expires_at = expires_in_secs.map(|secs| {
        (now + chrono::Duration::seconds(secs as i64)).to_rfc3339()
    });
    
    let result = sqlx::query(
        r#"
        INSERT INTO ground_items (room_id, item_id, x, y, dropped_by, dropped_at, expires_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(room_id as i64)
    .bind(item_id as i64)
    .bind(x as i64)
    .bind(y as i64)
    .bind(dropped_by)
    .bind(&dropped_at)
    .bind(expires_at)
    .execute(pool)
    .await?;
    
    Ok(result.last_insert_rowid())
}

/// Remove a ground item (picked up)
pub async fn remove_ground_item(pool: &DbPool, item_db_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM ground_items
        WHERE id = ?
        "#,
    )
    .bind(item_db_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get expired ground items (for notification before cleanup)
pub async fn get_expired_ground_items(pool: &DbPool) -> Result<Vec<GroundItem>, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query_as::<_, GroundItem>(
        r#"
        SELECT id, room_id, item_id, x, y, dropped_by, dropped_at, expires_at
        FROM ground_items
        WHERE expires_at IS NOT NULL AND expires_at <= ?
        "#,
    )
    .bind(now)
    .fetch_all(pool)
    .await
}

/// Remove expired ground items
pub async fn cleanup_expired_ground_items(pool: &DbPool) -> Result<u64, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        r#"
        DELETE FROM ground_items
        WHERE expires_at IS NOT NULL AND expires_at <= ?
        "#,
    )
    .bind(now)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

// =============================================================================
// Server State (Key-Value Store)
// =============================================================================

/// Get a server state value by key
pub async fn get_server_state(pool: &DbPool, key: &str) -> Result<Option<String>, sqlx::Error> {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM server_state WHERE key = ?"
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;
    
    Ok(result.map(|r| r.0))
}

/// Set a server state value
pub async fn set_server_state(pool: &DbPool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO server_state (key, value, updated_at)
        VALUES (?, ?, datetime('now'))
        ON CONFLICT (key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get the last shop restock date (YYYY-MM-DD format)
pub async fn get_last_restock_date(pool: &DbPool) -> Result<Option<String>, sqlx::Error> {
    get_server_state(pool, "last_restock_date").await
}

/// Set the last shop restock date (YYYY-MM-DD format)
pub async fn set_last_restock_date(pool: &DbPool, date: &str) -> Result<(), sqlx::Error> {
    set_server_state(pool, "last_restock_date", date).await
}
