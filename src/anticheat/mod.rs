//! Anti-cheat module for detecting and tracking game exploits
//!
//! This module provides:
//! - Movement validation (teleportation, speed hacking)
//! - Persistent hack tracking with escalating punishments
//! - Centralized hack reporting via `report_hack()`
//!
//! ## GML Parity
//! Based on GML `hack_alert.gml`:
//! - Persistent per-account hack counter (stored in database)
//! - MaxHacks (default 8) triggers permanent ban
//! - Logs to `logs/hacks/[Hacks]date.txt`
//! - Adds IP and MAC to ban lists on max hacks

mod hack_tracker;
pub mod movement;
pub mod types;

use std::collections::HashMap;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::config::AntiCheatConfig;
use crate::constants::{CHEAT_FLAGS_TO_BAN, CHEAT_FLAGS_TO_KICK};

pub use movement::{validate_position_bounds, MovementChecker};
pub use types::{CheatResult, HackResponse, HackType};

/// Central anti-cheat system
///
/// Combines movement validation with persistent hack tracking.
/// Use `report_hack()` for all hack detections to ensure consistent
/// logging and punishment escalation.
pub struct AntiCheat {
    /// Movement validation checker
    pub movement: MovementChecker,
    /// Session-based flag counts (for immediate kick/ban decisions)
    flagged: RwLock<HashMap<u64, u32>>,
}

impl AntiCheat {
    pub fn new() -> Self {
        Self {
            movement: MovementChecker::new(),
            flagged: RwLock::new(HashMap::new()),
        }
    }

    /// Initialize tracking for a player
    pub async fn init_player(&self, session_id: u64, x: u16, y: u16, room_id: u16) {
        self.movement.init_player(session_id, x, y, room_id).await;
    }

    /// Check a movement update
    pub async fn check_movement(
        &self,
        session_id: u64,
        new_x: u16,
        new_y: u16,
        room_id: u16,
    ) -> CheatResult {
        let result = self
            .movement
            .check_movement(session_id, new_x, new_y, room_id)
            .await;

        // Auto-flag on cheating detection
        if result.is_cheating() {
            self.flag_player(session_id).await;
        }

        result
    }

    /// Mark that a player is about to warp (legitimate teleport)
    pub async fn allow_warp(&self, session_id: u64) {
        self.movement.allow_warp(session_id).await;
    }

    /// Update player's room
    pub async fn set_room(&self, session_id: u64, room_id: u16, x: u16, y: u16) {
        self.movement.set_room(session_id, room_id, x, y).await;
    }

    /// Flag a player for suspicious activity (session-based)
    pub async fn flag_player(&self, session_id: u64) {
        let mut flagged = self.flagged.write().await;
        let count = flagged.entry(session_id).or_insert(0);
        *count += 1;
        info!(
            "Session {} flagged for suspicious activity (count: {})",
            session_id, *count
        );
    }

    /// Check if player should be kicked (session-based)
    pub async fn should_kick(&self, session_id: u64) -> bool {
        let flagged = self.flagged.read().await;
        flagged
            .get(&session_id)
            .map(|&c| c >= CHEAT_FLAGS_TO_KICK)
            .unwrap_or(false)
    }

    /// Check if player should be banned (session-based)
    pub async fn should_ban(&self, session_id: u64) -> bool {
        let flagged = self.flagged.read().await;
        flagged
            .get(&session_id)
            .map(|&c| c >= CHEAT_FLAGS_TO_BAN)
            .unwrap_or(false)
    }

    /// Get session flag count
    pub async fn get_flags(&self, session_id: u64) -> u32 {
        let flagged = self.flagged.read().await;
        flagged.get(&session_id).copied().unwrap_or(0)
    }

    /// Remove player tracking (on disconnect)
    pub async fn remove_player(&self, session_id: u64) {
        self.movement.remove_player(session_id).await;
        let mut flagged = self.flagged.write().await;
        flagged.remove(&session_id);
        debug!("Removed anti-cheat tracking for session {}", session_id);
    }

    /// Clean up stale entries
    pub async fn cleanup(&self) {
        self.movement.cleanup().await;
        let flagged = self.flagged.read().await;
        debug!(
            "Anti-cheat stats: {} flagged sessions",
            flagged.len()
        );
    }

    /// Report a hack attempt with persistent tracking
    ///
    /// This is the central method for all hack detection. It:
    /// 1. Logs to the hack_log database table
    /// 2. Optionally logs to file (logs/hacks/)
    /// 3. Increments the account's hack_count
    /// 4. Returns the appropriate action (log, warn, kick, ban)
    ///
    /// Use this for any detected exploit or suspicious activity.
    pub async fn report_hack(
        &self,
        pool: &SqlitePool,
        account_id: i64,
        character_id: Option<i64>,
        hack_type: HackType,
        description: &str,
        ip_address: Option<&str>,
        mac_address: Option<&str>,
        config: &AntiCheatConfig,
    ) -> HackResponse {
        hack_tracker::report_hack(
            pool,
            account_id,
            character_id,
            hack_type,
            description,
            ip_address,
            mac_address,
            config.max_hacks,
            config.log_to_file,
            &config.log_directory,
        )
        .await
    }

    /// Quick report method using default config values
    /// Use when config is not easily accessible
    pub async fn report_hack_simple(
        &self,
        pool: &SqlitePool,
        account_id: i64,
        character_id: Option<i64>,
        hack_type: HackType,
        description: &str,
        ip_address: Option<&str>,
        mac_address: Option<&str>,
    ) -> HackResponse {
        // Use hardcoded defaults when config not available
        const DEFAULT_MAX_HACKS: u32 = 8;
        const DEFAULT_LOG_TO_FILE: bool = true;
        const DEFAULT_LOG_DIR: &str = "logs/hacks";

        hack_tracker::report_hack(
            pool,
            account_id,
            character_id,
            hack_type,
            description,
            ip_address,
            mac_address,
            DEFAULT_MAX_HACKS,
            DEFAULT_LOG_TO_FILE,
            DEFAULT_LOG_DIR,
        )
        .await
    }
}

impl Default for AntiCheat {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create an Arc-wrapped AntiCheat
pub fn new_anticheat() -> Arc<AntiCheat> {
    Arc::new(AntiCheat::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_anticheat_init() {
        let ac = AntiCheat::new();
        ac.init_player(1, 100, 100, 1).await;

        // Should be clean initially
        assert!(!ac.should_kick(1).await);
        assert!(!ac.should_ban(1).await);
    }

    #[tokio::test]
    async fn test_flag_accumulation() {
        let ac = AntiCheat::new();
        ac.init_player(1, 100, 100, 1).await;

        // Flag multiple times
        for _ in 0..CHEAT_FLAGS_TO_KICK {
            ac.flag_player(1).await;
        }

        assert!(ac.should_kick(1).await);
    }
}
