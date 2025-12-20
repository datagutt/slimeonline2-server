//! Database layer for Slime Online 2 server
//!
//! Uses SQLite for local development with sqlx.

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::time::Duration;

mod accounts;
mod characters;
mod mail;

pub use accounts::*;
pub use characters::*;
pub use mail::*;

/// Database connection pool type.
pub type DbPool = SqlitePool;

/// Create a database connection pool.
pub async fn create_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(50)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .connect(database_url)
        .await
}

/// Initialize the database schema.
pub async fn init_database(pool: &DbPool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
