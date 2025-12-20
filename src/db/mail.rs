//! Mail database operations

use sqlx::FromRow;
use super::DbPool;

/// Mail record from database
#[derive(Debug, Clone, FromRow)]
pub struct Mail {
    pub id: i64,
    pub from_character_id: i64,
    pub to_character_id: i64,
    pub sender_name: String,
    pub message: String,
    pub item_id: i64,
    pub points: i64,
    pub is_read: i64,
    pub created_at: String,
}

/// Send mail from one character to another
/// Returns the new mail ID on success
pub async fn send_mail(
    pool: &DbPool,
    from_character_id: i64,
    to_character_id: i64,
    sender_name: &str,
    message: &str,
    item_id: i64,
    points: i64,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO mail (from_character_id, to_character_id, sender_name, message, item_id, points)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(from_character_id)
    .bind(to_character_id)
    .bind(sender_name)
    .bind(message)
    .bind(item_id)
    .bind(points)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Get mailbox for a character (paginated, newest first)
/// Returns up to 5 mails per page
pub async fn get_mailbox(
    pool: &DbPool,
    character_id: i64,
    page: i64,
) -> Result<Vec<Mail>, sqlx::Error> {
    let offset = page * 5;
    
    sqlx::query_as::<_, Mail>(
        r#"
        SELECT id, from_character_id, to_character_id, sender_name, message, 
               item_id, points, is_read, created_at
        FROM mail
        WHERE to_character_id = ?
        ORDER BY created_at DESC
        LIMIT 5 OFFSET ?
        "#,
    )
    .bind(character_id)
    .bind(offset)
    .fetch_all(pool)
    .await
}

/// Get total mail count for a character
pub async fn get_mail_count(
    pool: &DbPool,
    character_id: i64,
) -> Result<i64, sqlx::Error> {
    let result: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM mail WHERE to_character_id = ?
        "#,
    )
    .bind(character_id)
    .fetch_one(pool)
    .await?;

    Ok(result.0)
}

/// Get unread mail count for a character
pub async fn get_unread_mail_count(
    pool: &DbPool,
    character_id: i64,
) -> Result<i64, sqlx::Error> {
    let result: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM mail WHERE to_character_id = ? AND is_read = 0
        "#,
    )
    .bind(character_id)
    .fetch_one(pool)
    .await?;

    Ok(result.0)
}

/// Mark a mail as read
pub async fn mark_mail_read(
    pool: &DbPool,
    mail_id: i64,
    character_id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE mail SET is_read = 1 WHERE id = ? AND to_character_id = ?
        "#,
    )
    .bind(mail_id)
    .bind(character_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Get a specific mail (for claiming items/points)
pub async fn get_mail(
    pool: &DbPool,
    mail_id: i64,
    character_id: i64,
) -> Result<Option<Mail>, sqlx::Error> {
    sqlx::query_as::<_, Mail>(
        r#"
        SELECT id, from_character_id, to_character_id, sender_name, message, 
               item_id, points, is_read, created_at
        FROM mail
        WHERE id = ? AND to_character_id = ?
        "#,
    )
    .bind(mail_id)
    .bind(character_id)
    .fetch_optional(pool)
    .await
}

/// Delete a mail after claiming its contents
pub async fn delete_mail(
    pool: &DbPool,
    mail_id: i64,
    character_id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM mail WHERE id = ? AND to_character_id = ?
        "#,
    )
    .bind(mail_id)
    .bind(character_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Clear item/points from mail after claiming (but keep the message)
pub async fn clear_mail_attachments(
    pool: &DbPool,
    mail_id: i64,
    character_id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE mail SET item_id = 0, points = 0 WHERE id = ? AND to_character_id = ?
        "#,
    )
    .bind(mail_id)
    .bind(character_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}
