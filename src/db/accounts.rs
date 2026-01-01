//! Account database operations

use super::DbPool;
use sqlx::FromRow;

/// Account record from database
#[derive(Debug, Clone, FromRow)]
pub struct Account {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub mac_address: String,
    pub is_banned: bool,
    pub ban_reason: Option<String>,
    pub created_at: String,
    pub last_login: Option<String>,
}

/// Create a new account with hashed password.
pub async fn create_account(
    pool: &DbPool,
    username: &str,
    password_hash: &str,
    mac_address: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO accounts (username, password_hash, mac_address)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(username)
    .bind(password_hash)
    .bind(mac_address)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Find an account by username.
pub async fn find_account_by_username(
    pool: &DbPool,
    username: &str,
) -> Result<Option<Account>, sqlx::Error> {
    sqlx::query_as::<_, Account>(
        r#"
        SELECT id, username, password_hash, mac_address, is_banned, ban_reason, created_at, last_login
        FROM accounts
        WHERE username = ?
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

/// Check if a username already exists.
pub async fn username_exists(pool: &DbPool, username: &str) -> Result<bool, sqlx::Error> {
    let result =
        sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*) FROM accounts WHERE username = ?"#)
            .bind(username)
            .fetch_one(pool)
            .await?;

    Ok(result > 0)
}

/// Update last login time.
pub async fn update_last_login(pool: &DbPool, account_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE accounts
        SET last_login = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(account_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Check if an IP address is banned.
pub async fn is_ip_banned(pool: &DbPool, ip_address: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM bans 
        WHERE ban_type = 'ip' AND value = ?
        AND (expires_at IS NULL OR expires_at > datetime('now'))
        "#,
    )
    .bind(ip_address)
    .fetch_one(pool)
    .await?;

    Ok(result > 0)
}

/// Check if a MAC address is banned.
pub async fn is_mac_banned(pool: &DbPool, mac_address: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM bans 
        WHERE ban_type = 'mac' AND value = ?
        AND (expires_at IS NULL OR expires_at > datetime('now'))
        "#,
    )
    .bind(mac_address)
    .fetch_one(pool)
    .await?;

    Ok(result > 0)
}
