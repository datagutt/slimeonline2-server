//! Character database operations

use sqlx::FromRow;
use super::DbPool;

/// Character record from database
#[derive(Debug, Clone, FromRow)]
pub struct Character {
    pub id: i64,
    pub account_id: i64,
    pub username: String,
    pub x: i16,
    pub y: i16,
    pub room_id: i16,
    pub body_id: i16,
    pub acs1_id: i16,
    pub acs2_id: i16,
    pub points: i64,
    pub bank_balance: i64,
    pub trees_planted: i16,
    pub objects_built: i16,
    pub quest_id: i16,
    pub quest_step: i16,
    pub quest_var: i16,
    pub has_signature: bool,
    pub is_moderator: bool,
    pub clan_id: Option<i64>,
}

/// Inventory record from database
#[derive(Debug, Clone, FromRow)]
pub struct Inventory {
    pub character_id: i64,
    pub emote_1: i16, pub emote_2: i16, pub emote_3: i16, pub emote_4: i16, pub emote_5: i16,
    pub outfit_1: i16, pub outfit_2: i16, pub outfit_3: i16, pub outfit_4: i16, pub outfit_5: i16,
    pub outfit_6: i16, pub outfit_7: i16, pub outfit_8: i16, pub outfit_9: i16,
    pub accessory_1: i16, pub accessory_2: i16, pub accessory_3: i16, pub accessory_4: i16, pub accessory_5: i16,
    pub accessory_6: i16, pub accessory_7: i16, pub accessory_8: i16, pub accessory_9: i16,
    pub item_1: i16, pub item_2: i16, pub item_3: i16, pub item_4: i16, pub item_5: i16,
    pub item_6: i16, pub item_7: i16, pub item_8: i16, pub item_9: i16,
    pub tool_1: i16, pub tool_2: i16, pub tool_3: i16, pub tool_4: i16, pub tool_5: i16,
    pub tool_6: i16, pub tool_7: i16, pub tool_8: i16, pub tool_9: i16,
    pub equipped_tool: i16,
}

impl Inventory {
    /// Get emotes as array
    pub fn emotes(&self) -> [u8; 5] {
        [
            self.emote_1 as u8, self.emote_2 as u8, self.emote_3 as u8,
            self.emote_4 as u8, self.emote_5 as u8,
        ]
    }

    /// Get outfits as array
    pub fn outfits(&self) -> [u16; 9] {
        [
            self.outfit_1 as u16, self.outfit_2 as u16, self.outfit_3 as u16,
            self.outfit_4 as u16, self.outfit_5 as u16, self.outfit_6 as u16,
            self.outfit_7 as u16, self.outfit_8 as u16, self.outfit_9 as u16,
        ]
    }

    /// Get accessories as array
    pub fn accessories(&self) -> [u16; 9] {
        [
            self.accessory_1 as u16, self.accessory_2 as u16, self.accessory_3 as u16,
            self.accessory_4 as u16, self.accessory_5 as u16, self.accessory_6 as u16,
            self.accessory_7 as u16, self.accessory_8 as u16, self.accessory_9 as u16,
        ]
    }

    /// Get items as array
    pub fn items(&self) -> [u16; 9] {
        [
            self.item_1 as u16, self.item_2 as u16, self.item_3 as u16,
            self.item_4 as u16, self.item_5 as u16, self.item_6 as u16,
            self.item_7 as u16, self.item_8 as u16, self.item_9 as u16,
        ]
    }

    /// Get tools as array
    pub fn tools(&self) -> [u8; 9] {
        [
            self.tool_1 as u8, self.tool_2 as u8, self.tool_3 as u8,
            self.tool_4 as u8, self.tool_5 as u8, self.tool_6 as u8,
            self.tool_7 as u8, self.tool_8 as u8, self.tool_9 as u8,
        ]
    }
}

/// Create a new character for an account.
pub async fn create_character(
    pool: &DbPool,
    account_id: i64,
    username: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO characters (account_id, username)
        VALUES (?, ?)
        "#,
    )
    .bind(account_id)
    .bind(username)
    .execute(pool)
    .await?;

    let character_id = result.last_insert_rowid();

    // Create inventory for the character
    sqlx::query(
        r#"
        INSERT INTO inventories (character_id)
        VALUES (?)
        "#,
    )
    .bind(character_id)
    .execute(pool)
    .await?;

    Ok(character_id)
}

/// Find a character by account ID.
pub async fn find_character_by_account(
    pool: &DbPool,
    account_id: i64,
) -> Result<Option<Character>, sqlx::Error> {
    sqlx::query_as::<_, Character>(
        r#"
        SELECT id, account_id, username, x, y, room_id, body_id, acs1_id, acs2_id,
               points, bank_balance, trees_planted, objects_built, quest_id, quest_step,
               quest_var, has_signature, is_moderator, clan_id
        FROM characters
        WHERE account_id = ?
        "#,
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await
}

/// Get inventory for a character.
pub async fn get_inventory(
    pool: &DbPool,
    character_id: i64,
) -> Result<Option<Inventory>, sqlx::Error> {
    sqlx::query_as::<_, Inventory>(
        r#"
        SELECT *
        FROM inventories
        WHERE character_id = ?
        "#,
    )
    .bind(character_id)
    .fetch_optional(pool)
    .await
}

/// Update character position.
pub async fn update_position(
    pool: &DbPool,
    character_id: i64,
    x: i16,
    y: i16,
    room_id: i16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE characters
        SET x = ?, y = ?, room_id = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(x)
    .bind(y)
    .bind(room_id)
    .bind(character_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Update character points.
pub async fn update_points(
    pool: &DbPool,
    character_id: i64,
    points: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE characters
        SET points = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(points)
    .bind(character_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Update a specific item slot in inventory (slots 1-9)
pub async fn update_item_slot(
    pool: &DbPool,
    character_id: i64,
    slot: u8,
    item_id: i16,
) -> Result<(), sqlx::Error> {
    // Build the column name based on slot
    let column = match slot {
        1 => "item_1",
        2 => "item_2",
        3 => "item_3",
        4 => "item_4",
        5 => "item_5",
        6 => "item_6",
        7 => "item_7",
        8 => "item_8",
        9 => "item_9",
        _ => return Ok(()), // Invalid slot, silently ignore
    };

    let query = format!(
        "UPDATE inventories SET {} = ?, updated_at = datetime('now') WHERE character_id = ?",
        column
    );

    sqlx::query(&query)
        .bind(item_id)
        .bind(character_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Update a specific outfit slot in inventory (slots 1-9)
pub async fn update_outfit_slot(
    pool: &DbPool,
    character_id: i64,
    slot: u8,
    outfit_id: i16,
) -> Result<(), sqlx::Error> {
    let column = match slot {
        1 => "outfit_1",
        2 => "outfit_2",
        3 => "outfit_3",
        4 => "outfit_4",
        5 => "outfit_5",
        6 => "outfit_6",
        7 => "outfit_7",
        8 => "outfit_8",
        9 => "outfit_9",
        _ => return Ok(()),
    };

    let query = format!(
        "UPDATE inventories SET {} = ?, updated_at = datetime('now') WHERE character_id = ?",
        column
    );

    sqlx::query(&query)
        .bind(outfit_id)
        .bind(character_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Update a specific accessory slot in inventory (slots 1-9)
pub async fn update_accessory_slot(
    pool: &DbPool,
    character_id: i64,
    slot: u8,
    accessory_id: i16,
) -> Result<(), sqlx::Error> {
    let column = match slot {
        1 => "accessory_1",
        2 => "accessory_2",
        3 => "accessory_3",
        4 => "accessory_4",
        5 => "accessory_5",
        6 => "accessory_6",
        7 => "accessory_7",
        8 => "accessory_8",
        9 => "accessory_9",
        _ => return Ok(()),
    };

    let query = format!(
        "UPDATE inventories SET {} = ?, updated_at = datetime('now') WHERE character_id = ?",
        column
    );

    sqlx::query(&query)
        .bind(accessory_id)
        .bind(character_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Update a specific tool slot in inventory (slots 1-9)
pub async fn update_tool_slot(
    pool: &DbPool,
    character_id: i64,
    slot: u8,
    tool_id: i16,
) -> Result<(), sqlx::Error> {
    let column = match slot {
        1 => "tool_1",
        2 => "tool_2",
        3 => "tool_3",
        4 => "tool_4",
        5 => "tool_5",
        6 => "tool_6",
        7 => "tool_7",
        8 => "tool_8",
        9 => "tool_9",
        _ => return Ok(()),
    };

    let query = format!(
        "UPDATE inventories SET {} = ?, updated_at = datetime('now') WHERE character_id = ?",
        column
    );

    sqlx::query(&query)
        .bind(tool_id)
        .bind(character_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Update character's equipped body/outfit ID
pub async fn update_body_id(
    pool: &DbPool,
    character_id: i64,
    body_id: i16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE characters
        SET body_id = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(body_id)
    .bind(character_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Update character's equipped accessory 1 ID
pub async fn update_accessory1_id(
    pool: &DbPool,
    character_id: i64,
    acs1_id: i16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE characters
        SET acs1_id = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(acs1_id)
    .bind(character_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Update character's equipped accessory 2 ID
pub async fn update_accessory2_id(
    pool: &DbPool,
    character_id: i64,
    acs2_id: i16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE characters
        SET acs2_id = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(acs2_id)
    .bind(character_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get character's bank balance
pub async fn get_bank_balance(
    pool: &DbPool,
    character_id: i64,
) -> Result<i64, sqlx::Error> {
    let result: (i64,) = sqlx::query_as(
        r#"
        SELECT bank_balance FROM characters WHERE id = ?
        "#,
    )
    .bind(character_id)
    .fetch_one(pool)
    .await?;

    Ok(result.0)
}

/// Update character's bank balance
pub async fn update_bank_balance(
    pool: &DbPool,
    character_id: i64,
    bank_balance: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE characters
        SET bank_balance = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(bank_balance)
    .bind(character_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Update both points and bank balance atomically (for deposit/withdraw)
pub async fn update_points_and_bank(
    pool: &DbPool,
    character_id: i64,
    points: i64,
    bank_balance: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE characters
        SET points = ?, bank_balance = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(points)
    .bind(bank_balance)
    .bind(character_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Find character by username (for bank transfers)
pub async fn find_character_by_username(
    pool: &DbPool,
    username: &str,
) -> Result<Option<Character>, sqlx::Error> {
    sqlx::query_as::<_, Character>(
        r#"
        SELECT id, account_id, username, x, y, room_id, body_id, acs1_id, acs2_id,
               points, bank_balance, trees_planted, objects_built, quest_id, quest_step,
               quest_var, has_signature, is_moderator, clan_id
        FROM characters
        WHERE username = ?
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

/// Update character's equipped tool slot (0 = none, 1-9 = slot)
pub async fn update_equipped_tool(
    pool: &DbPool,
    character_id: i64,
    tool_slot: i16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE inventories
        SET equipped_tool = ?, updated_at = datetime('now')
        WHERE character_id = ?
        "#,
    )
    .bind(tool_slot)
    .bind(character_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Transfer funds between two bank accounts atomically
/// Uses a transaction to ensure both updates succeed or both fail
pub async fn transfer_bank_funds(
    pool: &DbPool,
    sender_id: i64,
    sender_new_balance: i64,
    receiver_id: i64,
    receiver_new_balance: i64,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Update sender's bank balance
    sqlx::query(
        r#"
        UPDATE characters
        SET bank_balance = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(sender_new_balance)
    .bind(sender_id)
    .execute(&mut *tx)
    .await?;

    // Update receiver's bank balance
    sqlx::query(
        r#"
        UPDATE characters
        SET bank_balance = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(receiver_new_balance)
    .bind(receiver_id)
    .execute(&mut *tx)
    .await?;

    // Commit transaction - if this fails, both updates are rolled back
    tx.commit().await?;

    Ok(())
}
