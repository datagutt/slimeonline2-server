//! Upgrader system database operations
//!
//! Handles persistence of upgrader progress, unlockable state, warp center unlocks,
//! music changer unlocks, and shop slot unlocks.

use sqlx::FromRow;

use super::DbPool;

// =============================================================================
// Upgrader State (Investment Progress)
// =============================================================================

#[derive(Debug, Clone, FromRow)]
pub struct UpgraderState {
    pub town_id: i64,
    pub category: String,
    pub slot_id: i64,
    pub paid: i64,
}

/// Get upgrader state for a specific slot
pub async fn get_upgrader_state(
    pool: &DbPool,
    town_id: u16,
    category: &str,
    slot_id: u8,
) -> Result<Option<UpgraderState>, sqlx::Error> {
    sqlx::query_as::<_, UpgraderState>(
        r#"
        SELECT town_id, category, slot_id, paid
        FROM upgrader_state
        WHERE town_id = ? AND category = ? AND slot_id = ?
        "#,
    )
    .bind(town_id as i64)
    .bind(category)
    .bind(slot_id as i64)
    .fetch_optional(pool)
    .await
}

/// Get all upgrader states for a town and category
pub async fn get_upgrader_states_by_category(
    pool: &DbPool,
    town_id: u16,
    category: &str,
) -> Result<Vec<UpgraderState>, sqlx::Error> {
    sqlx::query_as::<_, UpgraderState>(
        r#"
        SELECT town_id, category, slot_id, paid
        FROM upgrader_state
        WHERE town_id = ? AND category = ?
        ORDER BY slot_id
        "#,
    )
    .bind(town_id as i64)
    .bind(category)
    .fetch_all(pool)
    .await
}

/// Add investment points to an upgrade slot
pub async fn add_investment(
    pool: &DbPool,
    town_id: u16,
    category: &str,
    slot_id: u8,
    amount: u32,
) -> Result<u32, sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO upgrader_state (town_id, category, slot_id, paid)
        VALUES (?, ?, ?, ?)
        ON CONFLICT (town_id, category, slot_id) DO UPDATE SET
            paid = upgrader_state.paid + excluded.paid
        "#,
    )
    .bind(town_id as i64)
    .bind(category)
    .bind(slot_id as i64)
    .bind(amount as i64)
    .execute(pool)
    .await?;

    // Return new total paid
    let state = get_upgrader_state(pool, town_id, category, slot_id).await?;
    Ok(state.map(|s| s.paid as u32).unwrap_or(amount))
}

/// Get the amount already paid for an upgrade slot
pub async fn get_paid_amount(
    pool: &DbPool,
    town_id: u16,
    category: &str,
    slot_id: u8,
) -> Result<u32, sqlx::Error> {
    let state = get_upgrader_state(pool, town_id, category, slot_id).await?;
    Ok(state.map(|s| s.paid as u32).unwrap_or(0))
}

// =============================================================================
// Upgrader Unlocked State (Visibility of slots)
// =============================================================================

#[derive(Debug, Clone, FromRow)]
pub struct UpgraderUnlocked {
    pub town_id: i64,
    pub category: String,
    pub slot_id: i64,
    pub unlocked: i64,
}

/// Check if an upgrade slot is unlocked (visible to players)
pub async fn is_slot_unlocked(
    pool: &DbPool,
    town_id: u16,
    category: &str,
    slot_id: u8,
) -> Result<Option<bool>, sqlx::Error> {
    let result: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT unlocked
        FROM upgrader_unlocked
        WHERE town_id = ? AND category = ? AND slot_id = ?
        "#,
    )
    .bind(town_id as i64)
    .bind(category)
    .bind(slot_id as i64)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|(u,)| u == 1))
}

/// Set an upgrade slot as unlocked (visible)
pub async fn set_slot_unlocked(
    pool: &DbPool,
    town_id: u16,
    category: &str,
    slot_id: u8,
    unlocked: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO upgrader_unlocked (town_id, category, slot_id, unlocked)
        VALUES (?, ?, ?, ?)
        ON CONFLICT (town_id, category, slot_id) DO UPDATE SET
            unlocked = excluded.unlocked
        "#,
    )
    .bind(town_id as i64)
    .bind(category)
    .bind(slot_id as i64)
    .bind(if unlocked { 1i64 } else { 0i64 })
    .execute(pool)
    .await?;
    Ok(())
}

// =============================================================================
// Unlockable State (Objects in rooms like bubblegum machines)
// =============================================================================

#[derive(Debug, Clone, FromRow)]
pub struct UnlockableState {
    pub room_id: i64,
    pub unlockable_id: i64,
    pub available: i64,
}

/// Check if an unlockable object is available in a room
pub async fn is_unlockable_available(
    pool: &DbPool,
    room_id: u16,
    unlockable_id: u8,
) -> Result<bool, sqlx::Error> {
    let result: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT available
        FROM unlockable_state
        WHERE room_id = ? AND unlockable_id = ?
        "#,
    )
    .bind(room_id as i64)
    .bind(unlockable_id as i64)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|(a,)| a == 1).unwrap_or(false))
}

/// Get all available unlockables in a room
pub async fn get_room_unlockables(
    pool: &DbPool,
    room_id: u16,
) -> Result<Vec<u8>, sqlx::Error> {
    let results: Vec<(i64,)> = sqlx::query_as(
        r#"
        SELECT unlockable_id
        FROM unlockable_state
        WHERE room_id = ? AND available = 1
        ORDER BY unlockable_id
        "#,
    )
    .bind(room_id as i64)
    .fetch_all(pool)
    .await?;

    Ok(results.into_iter().map(|(id,)| id as u8).collect())
}

/// Set an unlockable object as available
pub async fn set_unlockable_available(
    pool: &DbPool,
    room_id: u16,
    unlockable_id: u8,
    available: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO unlockable_state (room_id, unlockable_id, available)
        VALUES (?, ?, ?)
        ON CONFLICT (room_id, unlockable_id) DO UPDATE SET
            available = excluded.available
        "#,
    )
    .bind(room_id as i64)
    .bind(unlockable_id as i64)
    .bind(if available { 1i64 } else { 0i64 })
    .execute(pool)
    .await?;
    Ok(())
}

// =============================================================================
// Music Changer State
// =============================================================================

#[derive(Debug, Clone, FromRow)]
pub struct MusicChangerState {
    pub room_id: i64,
    pub slot_id: i64,
    pub day_unlocked: i64,
    pub night_unlocked: i64,
}

/// Check if a music slot is unlocked (day or night)
pub async fn is_music_unlocked(
    pool: &DbPool,
    room_id: u16,
    slot_id: u8,
    is_day: bool,
) -> Result<bool, sqlx::Error> {
    let result: Option<MusicChangerState> = sqlx::query_as(
        r#"
        SELECT room_id, slot_id, day_unlocked, night_unlocked
        FROM music_changer_state
        WHERE room_id = ? AND slot_id = ?
        "#,
    )
    .bind(room_id as i64)
    .bind(slot_id as i64)
    .fetch_optional(pool)
    .await?;

    Ok(result
        .map(|s| {
            if is_day {
                s.day_unlocked == 1
            } else {
                s.night_unlocked == 1
            }
        })
        .unwrap_or(false))
}

/// Set a music slot as unlocked
pub async fn set_music_unlocked(
    pool: &DbPool,
    room_id: u16,
    slot_id: u8,
    is_day: bool,
) -> Result<(), sqlx::Error> {
    if is_day {
        sqlx::query(
            r#"
            INSERT INTO music_changer_state (room_id, slot_id, day_unlocked, night_unlocked)
            VALUES (?, ?, 1, 0)
            ON CONFLICT (room_id, slot_id) DO UPDATE SET
                day_unlocked = 1
            "#,
        )
        .bind(room_id as i64)
        .bind(slot_id as i64)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            r#"
            INSERT INTO music_changer_state (room_id, slot_id, day_unlocked, night_unlocked)
            VALUES (?, ?, 0, 1)
            ON CONFLICT (room_id, slot_id) DO UPDATE SET
                night_unlocked = 1
            "#,
        )
        .bind(room_id as i64)
        .bind(slot_id as i64)
        .execute(pool)
        .await?;
    }
    Ok(())
}

// =============================================================================
// Warp Center State
// =============================================================================

#[derive(Debug, Clone, FromRow)]
pub struct WarpCenterState {
    pub room_id: i64,
    pub slot_id: i64,
    pub warp_category: i64,
    pub unlocked: i64,
}

/// Check if a warp destination is unlocked
pub async fn is_warp_unlocked(
    pool: &DbPool,
    room_id: u16,
    slot_id: u8,
    warp_category: u8,
) -> Result<bool, sqlx::Error> {
    let result: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT unlocked
        FROM warp_center_state
        WHERE room_id = ? AND slot_id = ? AND warp_category = ?
        "#,
    )
    .bind(room_id as i64)
    .bind(slot_id as i64)
    .bind(warp_category as i64)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|(u,)| u == 1).unwrap_or(false))
}

/// Set a warp destination as unlocked
pub async fn set_warp_unlocked(
    pool: &DbPool,
    room_id: u16,
    slot_id: u8,
    warp_category: u8,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO warp_center_state (room_id, slot_id, warp_category, unlocked)
        VALUES (?, ?, ?, 1)
        ON CONFLICT (room_id, slot_id, warp_category) DO UPDATE SET
            unlocked = 1
        "#,
    )
    .bind(room_id as i64)
    .bind(slot_id as i64)
    .bind(warp_category as i64)
    .execute(pool)
    .await?;
    Ok(())
}

// =============================================================================
// Shop Slot Unlocked State
// =============================================================================

/// Check if a shop slot is unlocked (available for purchase)
pub async fn is_shop_slot_unlocked(
    pool: &DbPool,
    room_id: u16,
    slot_id: u8,
) -> Result<bool, sqlx::Error> {
    let result: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT available
        FROM shop_slot_unlocked
        WHERE room_id = ? AND slot_id = ?
        "#,
    )
    .bind(room_id as i64)
    .bind(slot_id as i64)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|(a,)| a == 1).unwrap_or(false))
}

/// Set a shop slot as unlocked
pub async fn set_shop_slot_unlocked(
    pool: &DbPool,
    room_id: u16,
    slot_id: u8,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO shop_slot_unlocked (room_id, slot_id, available)
        VALUES (?, ?, 1)
        ON CONFLICT (room_id, slot_id) DO UPDATE SET
            available = 1
        "#,
    )
    .bind(room_id as i64)
    .bind(slot_id as i64)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get the stock bonus for a room (from upgrader investments)
pub async fn get_shop_stock_bonus(
    pool: &DbPool,
    room_id: u16,
) -> Result<u16, sqlx::Error> {
    let result: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT bonus
        FROM shop_stock_bonus
        WHERE room_id = ?
        "#,
    )
    .bind(room_id as i64)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|(b,)| b as u16).unwrap_or(0))
}

/// Increase the permanent stock bonus for all items in a room (upgrade effect)
/// This bonus is added to max_stock from config for all limited items
pub async fn increase_shop_stock_bonus(
    pool: &DbPool,
    room_id: u16,
    increase_amount: u16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO shop_stock_bonus (room_id, bonus)
        VALUES (?, ?)
        ON CONFLICT (room_id) DO UPDATE SET
            bonus = bonus + ?
        "#,
    )
    .bind(room_id as i64)
    .bind(increase_amount as i64)
    .bind(increase_amount as i64)
    .execute(pool)
    .await?;
    Ok(())
}
