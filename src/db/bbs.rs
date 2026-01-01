//! BBS (Bulletin Board System) database operations

use super::DbPool;
use sqlx::FromRow;

/// BBS post record from database
#[derive(Debug, Clone, FromRow)]
pub struct BbsPost {
    pub id: i64,
    pub character_id: i64,
    pub category_id: i64,
    pub title: String,
    pub content: String,
    pub is_reported: i64,
    pub created_at: String,
}

/// BBS post summary (for list view - no content)
#[derive(Debug, Clone, FromRow)]
pub struct BbsPostSummary {
    pub id: i64,
    pub title: String,
    pub created_at: String,
}

/// Create a new BBS post
/// Returns the new post ID on success
pub async fn create_bbs_post(
    pool: &DbPool,
    character_id: i64,
    category_id: i64,
    title: &str,
    content: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO bbs_posts (character_id, category_id, title, content)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(character_id)
    .bind(category_id)
    .bind(title)
    .bind(content)
    .execute(pool)
    .await?;

    // Update the cooldown timestamp
    sqlx::query(
        r#"
        INSERT OR REPLACE INTO bbs_post_cooldowns (character_id, last_post_at)
        VALUES (?, datetime('now'))
        "#,
    )
    .bind(character_id)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Get BBS posts for a category (paginated, newest first)
/// Returns up to 4 posts per page (client displays 4 message slots)
pub async fn get_bbs_posts(
    pool: &DbPool,
    category_id: i64,
    page: i64,
) -> Result<Vec<BbsPostSummary>, sqlx::Error> {
    // Page is 1-based from client
    let offset = (page - 1) * 4;

    sqlx::query_as::<_, BbsPostSummary>(
        r#"
        SELECT id, title, created_at
        FROM bbs_posts
        WHERE category_id = ? AND is_reported = 0
        ORDER BY created_at DESC
        LIMIT 4 OFFSET ?
        "#,
    )
    .bind(category_id)
    .bind(offset)
    .fetch_all(pool)
    .await
}

/// Get total page count for a category
/// 4 posts per page
pub async fn get_bbs_page_count(pool: &DbPool, category_id: i64) -> Result<i64, sqlx::Error> {
    let result: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM bbs_posts WHERE category_id = ? AND is_reported = 0
        "#,
    )
    .bind(category_id)
    .fetch_one(pool)
    .await?;

    // Calculate pages (4 posts per page, minimum 0)
    let total_posts = result.0;
    let pages = (total_posts + 3) / 4; // Ceiling division
    Ok(pages)
}

/// Get a specific BBS post with full content
pub async fn get_bbs_post(pool: &DbPool, post_id: i64) -> Result<Option<BbsPost>, sqlx::Error> {
    sqlx::query_as::<_, BbsPost>(
        r#"
        SELECT id, character_id, category_id, title, content, is_reported, created_at
        FROM bbs_posts
        WHERE id = ?
        "#,
    )
    .bind(post_id)
    .fetch_optional(pool)
    .await
}

/// Get poster's username for a post
pub async fn get_bbs_post_poster_name(
    pool: &DbPool,
    post_id: i64,
) -> Result<Option<String>, sqlx::Error> {
    let result: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT c.username 
        FROM bbs_posts p
        JOIN characters c ON p.character_id = c.id
        WHERE p.id = ?
        "#,
    )
    .bind(post_id)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|r| r.0))
}

/// Report a BBS post (flag for moderation)
pub async fn report_bbs_post(pool: &DbPool, post_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE bbs_posts SET is_reported = 1 WHERE id = ?
        "#,
    )
    .bind(post_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Delete a BBS post (owner only)
pub async fn delete_bbs_post(
    pool: &DbPool,
    post_id: i64,
    character_id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM bbs_posts WHERE id = ? AND character_id = ?
        "#,
    )
    .bind(post_id)
    .bind(character_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Check if user can post (cooldown expired)
/// Returns true if user can post, false if still on cooldown
pub async fn can_post_bbs(
    pool: &DbPool,
    character_id: i64,
    cooldown_seconds: i64,
) -> Result<bool, sqlx::Error> {
    let result: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT (strftime('%s', 'now') - strftime('%s', last_post_at)) as seconds_since
        FROM bbs_post_cooldowns
        WHERE character_id = ?
        "#,
    )
    .bind(character_id)
    .fetch_optional(pool)
    .await?;

    match result {
        Some((seconds_since,)) => Ok(seconds_since >= cooldown_seconds),
        None => Ok(true), // No previous post, can post
    }
}
