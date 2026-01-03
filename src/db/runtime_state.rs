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
pub async fn get_plant_states(pool: &DbPool, room_id: u16) -> Result<Vec<PlantState>, sqlx::Error> {
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
pub async fn harvest_plant(pool: &DbPool, room_id: u16, spot_id: u8) -> Result<(), sqlx::Error> {
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
pub async fn clear_plant(pool: &DbPool, room_id: u16, spot_id: u8) -> Result<(), sqlx::Error> {
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
pub async fn get_shop_stock(pool: &DbPool, room_id: u16) -> Result<Vec<ShopStock>, sqlx::Error> {
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
pub async fn get_ground_items(pool: &DbPool, room_id: u16) -> Result<Vec<GroundItem>, sqlx::Error> {
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
    let expires_at =
        expires_in_secs.map(|secs| (now + chrono::Duration::seconds(secs as i64)).to_rfc3339());

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
    let result: Option<(String,)> = sqlx::query_as("SELECT value FROM server_state WHERE key = ?")
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

/// Update plant next stage time
pub async fn update_plant_next_stage(
    pool: &DbPool,
    room_id: u16,
    spot_id: u8,
    next_stage_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE plant_state
        SET next_stage_at = ?
        WHERE room_id = ? AND spot_id = ?
        "#,
    )
    .bind(next_stage_at.to_rfc3339())
    .bind(room_id as i64)
    .bind(spot_id as i64)
    .execute(pool)
    .await?;
    Ok(())
}

/// Take fruit from a plant (sets the fruit slot to 0)
/// For simplicity, we track all 3 fruits via the has_fruit field.
/// When all fruits are taken, has_fruit becomes 0.
pub async fn take_plant_fruit(
    pool: &DbPool,
    room_id: u16,
    spot_id: u8,
    _fruit_slot: u8,
) -> Result<(), sqlx::Error> {
    // For now, just mark has_fruit as 0 when any fruit is taken
    // A more complex implementation would track individual fruits
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

// =============================================================================
// Storage Extension
// =============================================================================

/// Get storage contents for a category (180 slots max)
pub async fn get_storage(
    pool: &DbPool,
    character_id: i64,
    category: u8,
) -> Result<Vec<u16>, sqlx::Error> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT slot, item_id FROM storage WHERE character_id = ? AND category = ? ORDER BY slot",
    )
    .bind(character_id)
    .bind(category as i64)
    .fetch_all(pool)
    .await?;

    // Build result with 180 slots
    let mut result = vec![0u16; 180];
    for (slot, item_id) in rows {
        if slot >= 0 && (slot as usize) < result.len() {
            result[slot as usize] = item_id as u16;
        }
    }
    Ok(result)
}

/// Save storage contents for a category
pub async fn save_storage(
    pool: &DbPool,
    character_id: i64,
    category: u8,
    items: &[u16],
) -> Result<(), sqlx::Error> {
    // Delete existing entries
    sqlx::query("DELETE FROM storage WHERE character_id = ? AND category = ?")
        .bind(character_id)
        .bind(category as i64)
        .execute(pool)
        .await?;

    // Insert non-zero items
    for (slot, &item_id) in items.iter().enumerate() {
        if item_id != 0 {
            sqlx::query(
                "INSERT INTO storage (character_id, category, slot, item_id) VALUES (?, ?, ?, ?)",
            )
            .bind(character_id)
            .bind(category as i64)
            .bind(slot as i64)
            .bind(item_id as i64)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

// =============================================================================
// One-Time Items
// =============================================================================

#[derive(Debug, Clone)]
pub struct OneTimeItem {
    pub category: u8,
    pub item_id: u16,
}

/// Get one-time item definition
pub async fn get_one_time_item(
    pool: &DbPool,
    room_id: u16,
    real_id: u8,
) -> Result<Option<OneTimeItem>, sqlx::Error> {
    let row: Option<(i64, i64)> = sqlx::query_as(
        "SELECT category, item_id FROM one_time_items WHERE room_id = ? AND real_id = ?",
    )
    .bind(room_id as i64)
    .bind(real_id as i64)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(cat, id)| OneTimeItem {
        category: cat as u8,
        item_id: id as u16,
    }))
}

/// Check if player has taken a one-time item
pub async fn has_taken_one_time(
    pool: &DbPool,
    character_id: i64,
    room_id: u16,
    real_id: u8,
) -> Result<bool, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM one_time_taken WHERE character_id = ? AND room_id = ? AND real_id = ?",
    )
    .bind(character_id)
    .bind(room_id as i64)
    .bind(real_id as i64)
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some())
}

/// Mark a one-time item as taken
pub async fn mark_one_time_taken(
    pool: &DbPool,
    character_id: i64,
    room_id: u16,
    real_id: u8,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR IGNORE INTO one_time_taken (character_id, room_id, real_id) VALUES (?, ?, ?)",
    )
    .bind(character_id)
    .bind(room_id as i64)
    .bind(real_id as i64)
    .execute(pool)
    .await?;
    Ok(())
}

// =============================================================================
// Racing
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct RaceRecord {
    pub name: String,
    pub time_ms: u32,
}

#[derive(Debug, Clone, Default)]
pub struct RaceRecords {
    pub single_records: Vec<RaceRecord>,
    pub clan_records: Vec<RaceRecord>,
}

/// Get race records for a race
pub async fn get_race_records(pool: &DbPool, race_id: u8) -> Result<RaceRecords, sqlx::Error> {
    let singles: Vec<(String, i64)> = sqlx::query_as(
        "SELECT name, time_ms FROM race_records WHERE race_id = ? AND record_type = 'single' ORDER BY rank",
    )
    .bind(race_id as i64)
    .fetch_all(pool)
    .await?;

    let clans: Vec<(String, i64)> = sqlx::query_as(
        "SELECT name, time_ms FROM race_records WHERE race_id = ? AND record_type = 'clan' ORDER BY rank",
    )
    .bind(race_id as i64)
    .fetch_all(pool)
    .await?;

    Ok(RaceRecords {
        single_records: singles
            .into_iter()
            .map(|(name, time)| RaceRecord {
                name,
                time_ms: time as u32,
            })
            .collect(),
        clan_records: clans
            .into_iter()
            .map(|(name, time)| RaceRecord {
                name,
                time_ms: time as u32,
            })
            .collect(),
    })
}

/// Get race time limit
pub async fn get_race_time_limit(pool: &DbPool, race_id: u8) -> Result<u32, sqlx::Error> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT time_limit_ms FROM race_config WHERE race_id = ?")
            .bind(race_id as i64)
            .fetch_optional(pool)
            .await?;

    Ok(row.map(|(t,)| t as u32).unwrap_or(0))
}

/// Submit a race record (inserts if in top 10)
pub async fn submit_race_record(
    pool: &DbPool,
    race_id: u8,
    name: &str,
    time_ms: u32,
    _character_id: i64,
) -> Result<Option<u8>, sqlx::Error> {
    // Get current 10th place time
    let tenth: Option<(i64,)> = sqlx::query_as(
        "SELECT time_ms FROM race_records WHERE race_id = ? AND record_type = 'single' AND rank = 10",
    )
    .bind(race_id as i64)
    .fetch_optional(pool)
    .await?;

    let tenth_time = tenth.map(|(t,)| t as u32).unwrap_or(u32::MAX);

    if time_ms >= tenth_time {
        return Ok(None); // Didn't make top 10
    }

    // Find the rank this time would be
    let better_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM race_records WHERE race_id = ? AND record_type = 'single' AND time_ms < ?",
    )
    .bind(race_id as i64)
    .bind(time_ms as i64)
    .fetch_one(pool)
    .await?;

    let new_rank = (better_count.0 + 1) as u8;

    // Shift down all records at or below this rank
    sqlx::query(
        "UPDATE race_records SET rank = rank + 1 WHERE race_id = ? AND record_type = 'single' AND rank >= ?",
    )
    .bind(race_id as i64)
    .bind(new_rank as i64)
    .execute(pool)
    .await?;

    // Delete 11th place if it exists
    sqlx::query("DELETE FROM race_records WHERE race_id = ? AND record_type = 'single' AND rank > 10")
        .bind(race_id as i64)
        .execute(pool)
        .await?;

    // Insert new record
    sqlx::query(
        "INSERT INTO race_records (race_id, record_type, rank, name, time_ms) VALUES (?, 'single', ?, ?, ?)",
    )
    .bind(race_id as i64)
    .bind(new_rank as i64)
    .bind(name)
    .bind(time_ms as i64)
    .execute(pool)
    .await?;

    Ok(Some(new_rank))
}

// =============================================================================
// Music Changer
// =============================================================================

#[derive(Debug, Clone)]
pub struct MusicChangerState {
    pub current_day_music: u8,
    pub current_night_music: u8,
    pub day_track_1: u8,
    pub day_track_2: u8,
    pub day_track_3: u8,
    pub day_track_3_unlocked: bool,
    pub night_track_1: u8,
    pub night_track_2: u8,
    pub night_track_3: u8,
    pub night_track_3_unlocked: bool,
    pub cooldown_until: Option<String>,
}

impl MusicChangerState {
    pub fn is_on_cooldown(&self) -> bool {
        if let Some(ref until) = self.cooldown_until {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(until) {
                return dt > Utc::now();
            }
        }
        false
    }
}

/// Get music changer state for a room
pub async fn get_music_changer_state(
    pool: &DbPool,
    room_id: u16,
) -> Result<Option<MusicChangerState>, sqlx::Error> {
    let row: Option<(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, Option<String>)> =
        sqlx::query_as(
            r#"SELECT current_day_music, current_night_music,
                      day_track_1, day_track_2, day_track_3, day_track_3_unlocked,
                      night_track_1, night_track_2, night_track_3, night_track_3_unlocked,
                      cooldown_until
               FROM music_changer_state WHERE room_id = ?"#,
        )
        .bind(room_id as i64)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| MusicChangerState {
        current_day_music: r.0 as u8,
        current_night_music: r.1 as u8,
        day_track_1: r.2 as u8,
        day_track_2: r.3 as u8,
        day_track_3: r.4 as u8,
        day_track_3_unlocked: r.5 != 0,
        night_track_1: r.6 as u8,
        night_track_2: r.7 as u8,
        night_track_3: r.8 as u8,
        night_track_3_unlocked: r.9 != 0,
        cooldown_until: r.10,
    }))
}

/// Set room music
pub async fn set_room_music(
    pool: &DbPool,
    room_id: u16,
    track: u8,
    music_id: u8,
    cooldown_secs: i64,
) -> Result<(), sqlx::Error> {
    let cooldown_until = (Utc::now() + chrono::Duration::seconds(cooldown_secs)).to_rfc3339();

    let column = if track <= 3 {
        "current_day_music"
    } else {
        "current_night_music"
    };

    let query = format!(
        "UPDATE music_changer_state SET {} = ?, cooldown_until = ? WHERE room_id = ?",
        column
    );

    sqlx::query(&query)
        .bind(music_id as i64)
        .bind(&cooldown_until)
        .bind(room_id as i64)
        .execute(pool)
        .await?;

    Ok(())
}
