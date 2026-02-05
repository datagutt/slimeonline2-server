//! Persistent hack tracking for anti-cheat
//!
//! Tracks hack attempts per account and handles escalating punishments.
//! Based on GML hack_alert.gml behavior (MaxHacks = 8 triggers permanent ban).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use chrono::{Local, Utc};
use sqlx::SqlitePool;
use tracing::{error, info, warn};

use super::types::{HackResponse, HackType};

/// Log a hack attempt to the database
pub async fn log_hack_to_db(
    pool: &SqlitePool,
    account_id: i64,
    character_id: Option<i64>,
    hack_type: HackType,
    description: &str,
    ip_address: Option<&str>,
    mac_address: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO hack_log (account_id, character_id, hack_type, description, ip_address, mac_address)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(account_id)
    .bind(character_id)
    .bind(hack_type.to_string())
    .bind(description)
    .bind(ip_address)
    .bind(mac_address)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Increment the hack count for an account
pub async fn increment_hack_count(pool: &SqlitePool, account_id: i64) -> Result<u32, sqlx::Error> {
    // Increment and return new count
    sqlx::query(
        r#"
        UPDATE accounts SET hack_count = hack_count + 1 WHERE id = ?
        "#,
    )
    .bind(account_id)
    .execute(pool)
    .await?;

    // Get the new count
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT hack_count FROM accounts WHERE id = ?
        "#,
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;

    Ok(count.0 as u32)
}

/// Get the current hack count for an account
pub async fn get_hack_count(pool: &SqlitePool, account_id: i64) -> Result<u32, sqlx::Error> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT hack_count FROM accounts WHERE id = ?
        "#,
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;

    Ok(count.0 as u32)
}

/// Reset the hack count for an account (admin action)
pub async fn reset_hack_count(pool: &SqlitePool, account_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE accounts SET hack_count = 0 WHERE id = ?
        "#,
    )
    .bind(account_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Add an IP address to the ban list
pub async fn ban_ip(pool: &SqlitePool, ip_address: &str, reason: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO bans (type, value, reason, created_at)
        VALUES ('ip', ?, ?, datetime('now'))
        "#,
    )
    .bind(ip_address)
    .bind(reason)
    .execute(pool)
    .await?;

    Ok(())
}

/// Add a MAC address to the ban list
pub async fn ban_mac(pool: &SqlitePool, mac_address: &str, reason: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO bans (type, value, reason, created_at)
        VALUES ('mac', ?, ?, datetime('now'))
        "#,
    )
    .bind(mac_address)
    .bind(reason)
    .execute(pool)
    .await?;

    Ok(())
}

/// Ban an account by ID
pub async fn ban_account(pool: &SqlitePool, account_id: i64, reason: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE accounts SET is_banned = 1, ban_reason = ? WHERE id = ?
        "#,
    )
    .bind(reason)
    .bind(account_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Log a hack to file (GML behavior: srvr_logs/[Hacks]date.txt)
pub fn log_hack_to_file(
    log_directory: &str,
    account_id: i64,
    character_id: Option<i64>,
    hack_type: HackType,
    description: &str,
    ip_address: Option<&str>,
    mac_address: Option<&str>,
) {
    // Create log directory if needed
    if let Err(e) = fs::create_dir_all(log_directory) {
        error!("Failed to create hack log directory: {}", e);
        return;
    }

    // Build filename: [Hacks]YYYY-MM-DD.txt
    let date = Local::now().format("%Y-%m-%d");
    let filename = format!("[Hacks]{}.txt", date);
    let filepath = Path::new(log_directory).join(filename);

    // Build log line
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    let char_str = character_id
        .map(|c| c.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    let ip_str = ip_address.unwrap_or("N/A");
    let mac_str = mac_address.unwrap_or("N/A");

    let log_line = format!(
        "[{}] Account:{} Char:{} Type:{} IP:{} MAC:{} - {}\n",
        timestamp, account_id, char_str, hack_type, ip_str, mac_str, description
    );

    // Append to file
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&filepath)
    {
        Ok(mut file) => {
            if let Err(e) = file.write_all(log_line.as_bytes()) {
                error!("Failed to write to hack log: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to open hack log file: {}", e);
        }
    }
}

/// Report a hack and determine the appropriate response
///
/// This is the central method for handling all hack detections.
/// It logs to database (and optionally file), increments the hack counter,
/// and returns the appropriate action to take.
pub async fn report_hack(
    pool: &SqlitePool,
    account_id: i64,
    character_id: Option<i64>,
    hack_type: HackType,
    description: &str,
    ip_address: Option<&str>,
    mac_address: Option<&str>,
    max_hacks: u32,
    log_to_file: bool,
    log_directory: &str,
) -> HackResponse {
    // Log to database
    if let Err(e) = log_hack_to_db(
        pool,
        account_id,
        character_id,
        hack_type,
        description,
        ip_address,
        mac_address,
    )
    .await
    {
        error!("Failed to log hack to database: {}", e);
    }

    // Log to file if enabled
    if log_to_file {
        log_hack_to_file(
            log_directory,
            account_id,
            character_id,
            hack_type,
            description,
            ip_address,
            mac_address,
        );
    }

    // Increment hack count
    let hack_count = match increment_hack_count(pool, account_id).await {
        Ok(count) => count,
        Err(e) => {
            error!("Failed to increment hack count: {}", e);
            // Continue with unknown count
            0
        }
    };

    warn!(
        "Hack detected: account={} type={} count={}/{} - {}",
        account_id, hack_type, hack_count, max_hacks, description
    );

    // Determine response based on hack count
    if hack_count >= max_hacks {
        // Ban the account
        let ban_reason = format!(
            "Maximum hack attempts reached ({}/{}). Last: {}",
            hack_count, max_hacks, hack_type
        );

        if let Err(e) = ban_account(pool, account_id, &ban_reason).await {
            error!("Failed to ban account: {}", e);
        }

        // Also ban IP if available
        if let Some(ip) = ip_address {
            if let Err(e) = ban_ip(pool, ip, &ban_reason).await {
                error!("Failed to ban IP: {}", e);
            }
        }

        // Also ban MAC if available
        if let Some(mac) = mac_address {
            if let Err(e) = ban_mac(pool, mac, &ban_reason).await {
                error!("Failed to ban MAC: {}", e);
            }
        }

        info!(
            "Account {} permanently banned after {} hack attempts",
            account_id, hack_count
        );

        HackResponse::Ban {
            reason: ban_reason,
        }
    } else if hack_count >= max_hacks / 2 {
        // Warning threshold (halfway to ban)
        HackResponse::Warning {
            hack_count,
            max_hacks,
        }
    } else {
        // Just log for now
        HackResponse::Logged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hack_type_display() {
        assert_eq!(HackType::InsufficientFunds.to_string(), "InsufficientFunds");
        assert_eq!(HackType::SpeedHack.to_string(), "SpeedHack");
    }

    #[test]
    fn test_hack_response_should_disconnect() {
        assert!(!HackResponse::Logged.should_disconnect());
        assert!(!HackResponse::Warning {
            hack_count: 1,
            max_hacks: 8
        }
        .should_disconnect());
        assert!(HackResponse::Kick {
            reason: "test".to_string()
        }
        .should_disconnect());
        assert!(HackResponse::Ban {
            reason: "test".to_string()
        }
        .should_disconnect());
    }
}
