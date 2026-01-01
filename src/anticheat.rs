//! Anti-cheat module for detecting game exploits
//!
//! Focuses on actual exploits possible in this game:
//! - Teleportation (moving faster than physics allow)
//! - Speed hacking
//! - Position spoofing

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::constants::{
    CHEAT_FLAGS_TO_BAN, CHEAT_FLAGS_TO_KICK, CHEAT_VIOLATION_THRESHOLD,
    CHEAT_VIOLATION_WINDOW_SECS, MAX_MOVEMENT_DISTANCE_PER_UPDATE, MAX_PLAYER_SPEED, MAX_ROOM_X,
    MAX_ROOM_Y,
};

/// Cheat detection result
#[derive(Debug, Clone)]
pub enum CheatResult {
    /// No cheating detected
    Clean,
    /// Suspicious but not definitive
    Suspicious { reason: String, severity: u8 },
    /// Definite cheat detected
    Cheating { reason: String },
}

impl CheatResult {
    pub fn is_clean(&self) -> bool {
        matches!(self, CheatResult::Clean)
    }

    pub fn is_cheating(&self) -> bool {
        matches!(self, CheatResult::Cheating { .. })
    }
}

/// Player position history for movement analysis
#[derive(Debug, Clone)]
struct PositionHistory {
    /// Recent positions with timestamps
    positions: Vec<(u16, u16, Instant)>,
    /// Last known room
    room_id: u16,
    /// Violation count in current window
    violations: Vec<(Instant, String)>,
    /// Whether player is currently warping (legitimate teleport)
    is_warping: bool,
    /// Time of last warp
    last_warp: Option<Instant>,
}

impl PositionHistory {
    fn new(x: u16, y: u16, room_id: u16) -> Self {
        Self {
            positions: vec![(x, y, Instant::now())],
            room_id,
            violations: Vec::new(),
            is_warping: false,
            last_warp: None,
        }
    }

    /// Add a position update and check for teleportation
    fn update(&mut self, new_x: u16, new_y: u16, new_room: u16) -> CheatResult {
        let now = Instant::now();

        // Room change is handled separately (warps, doors, etc.)
        if new_room != self.room_id {
            self.room_id = new_room;
            self.positions.clear();
            self.positions.push((new_x, new_y, now));
            self.is_warping = false;
            return CheatResult::Clean;
        }

        // If warping flag is set, allow the position change
        if self.is_warping {
            self.positions.clear();
            self.positions.push((new_x, new_y, now));
            self.is_warping = false;
            self.last_warp = Some(now);
            return CheatResult::Clean;
        }

        // Get last position
        if let Some(&(last_x, last_y, last_time)) = self.positions.last() {
            let elapsed = now.duration_since(last_time);

            // Calculate distance moved
            let dx = (new_x as f64 - last_x as f64).abs();
            let dy = (new_y as f64 - last_y as f64).abs();
            let distance = (dx * dx + dy * dy).sqrt();

            // Check for teleportation
            if distance > MAX_MOVEMENT_DISTANCE_PER_UPDATE {
                // Could be legitimate lag - check if it's extreme
                let elapsed_secs = elapsed.as_secs_f64().max(0.001);
                let speed = distance / elapsed_secs;

                // If speed is impossibly high even accounting for lag
                if speed > MAX_PLAYER_SPEED * 10.0 {
                    let reason = format!(
                        "Teleport detected: moved {} pixels in {:.2}s (speed: {:.0})",
                        distance as u32, elapsed_secs, speed
                    );
                    self.add_violation(&reason);

                    // Check if we've hit the threshold
                    if self.violation_count() >= CHEAT_VIOLATION_THRESHOLD {
                        return CheatResult::Cheating { reason };
                    }
                    return CheatResult::Suspicious {
                        reason,
                        severity: 3,
                    };
                }
            }

            // Check for speed hacking (sustained high speed)
            if elapsed.as_millis() > 100 {
                let elapsed_secs = elapsed.as_secs_f64();
                let speed = distance / elapsed_secs;

                if speed > MAX_PLAYER_SPEED * 2.0 {
                    let reason = format!(
                        "Speed hack suspected: {:.0} pixels/sec (max: {:.0})",
                        speed, MAX_PLAYER_SPEED
                    );
                    self.add_violation(&reason);

                    if self.violation_count() >= CHEAT_VIOLATION_THRESHOLD {
                        return CheatResult::Cheating { reason };
                    }
                    return CheatResult::Suspicious {
                        reason,
                        severity: 2,
                    };
                }
            }
        }

        // Record position (keep last 10)
        self.positions.push((new_x, new_y, now));
        if self.positions.len() > 10 {
            self.positions.remove(0);
        }

        CheatResult::Clean
    }

    fn add_violation(&mut self, reason: &str) {
        let now = Instant::now();
        self.violations.push((now, reason.to_string()));

        // Clean old violations
        let cutoff = now - Duration::from_secs(CHEAT_VIOLATION_WINDOW_SECS);
        self.violations.retain(|(t, _)| *t > cutoff);
    }

    fn violation_count(&self) -> u32 {
        let cutoff = Instant::now() - Duration::from_secs(CHEAT_VIOLATION_WINDOW_SECS);
        self.violations.iter().filter(|(t, _)| *t > cutoff).count() as u32
    }

    /// Mark that a legitimate warp is about to happen
    fn set_warping(&mut self) {
        self.is_warping = true;
    }
}

/// Anti-cheat system
pub struct AntiCheat {
    /// Position tracking per player (session_id -> history)
    players: RwLock<HashMap<u64, PositionHistory>>,
    /// Flagged players (session_id -> flag count)
    flagged: RwLock<HashMap<u64, u32>>,
}

impl AntiCheat {
    pub fn new() -> Self {
        Self {
            players: RwLock::new(HashMap::new()),
            flagged: RwLock::new(HashMap::new()),
        }
    }

    /// Initialize tracking for a player
    pub async fn init_player(&self, session_id: u64, x: u16, y: u16, room_id: u16) {
        let mut players = self.players.write().await;
        players.insert(session_id, PositionHistory::new(x, y, room_id));
        debug!("Initialized anti-cheat tracking for session {}", session_id);
    }

    /// Check a movement update
    pub async fn check_movement(
        &self,
        session_id: u64,
        new_x: u16,
        new_y: u16,
        room_id: u16,
    ) -> CheatResult {
        let mut players = self.players.write().await;

        let history = match players.get_mut(&session_id) {
            Some(h) => h,
            None => {
                // Player not tracked yet, initialize
                players.insert(session_id, PositionHistory::new(new_x, new_y, room_id));
                return CheatResult::Clean;
            }
        };

        let result = history.update(new_x, new_y, room_id);

        // Log and flag if cheating detected
        match &result {
            CheatResult::Suspicious { reason, severity } => {
                debug!(
                    "Suspicious activity from session {}: {} (severity: {})",
                    session_id, reason, severity
                );
            }
            CheatResult::Cheating { reason } => {
                warn!("Cheat detected from session {}: {}", session_id, reason);
                drop(players);
                self.flag_player(session_id).await;
            }
            CheatResult::Clean => {}
        }

        result
    }

    /// Mark that a player is about to warp (legitimate teleport)
    pub async fn allow_warp(&self, session_id: u64) {
        let mut players = self.players.write().await;
        if let Some(history) = players.get_mut(&session_id) {
            history.set_warping();
            debug!("Allowing warp for session {}", session_id);
        }
    }

    /// Update player's room (for room changes via doors, etc.)
    pub async fn set_room(&self, session_id: u64, room_id: u16, x: u16, y: u16) {
        let mut players = self.players.write().await;
        if let Some(history) = players.get_mut(&session_id) {
            history.room_id = room_id;
            history.positions.clear();
            history.positions.push((x, y, Instant::now()));
        }
    }

    /// Flag a player for cheating
    async fn flag_player(&self, session_id: u64) {
        let mut flagged = self.flagged.write().await;
        let count = flagged.entry(session_id).or_insert(0);
        *count += 1;
        info!(
            "Session {} flagged for cheating (count: {})",
            session_id, *count
        );
    }

    /// Check if player should be kicked
    pub async fn should_kick(&self, session_id: u64) -> bool {
        let flagged = self.flagged.read().await;
        flagged
            .get(&session_id)
            .map(|&c| c >= CHEAT_FLAGS_TO_KICK)
            .unwrap_or(false)
    }

    /// Check if player should be banned
    pub async fn should_ban(&self, session_id: u64) -> bool {
        let flagged = self.flagged.read().await;
        flagged
            .get(&session_id)
            .map(|&c| c >= CHEAT_FLAGS_TO_BAN)
            .unwrap_or(false)
    }

    /// Get flag count for a player
    pub async fn get_flags(&self, session_id: u64) -> u32 {
        let flagged = self.flagged.read().await;
        flagged.get(&session_id).copied().unwrap_or(0)
    }

    /// Remove player tracking (on disconnect)
    pub async fn remove_player(&self, session_id: u64) {
        let mut players = self.players.write().await;
        players.remove(&session_id);
        debug!("Removed anti-cheat tracking for session {}", session_id);
    }

    /// Clean up stale entries
    pub async fn cleanup(&self) {
        // For now, just log stats
        let players = self.players.read().await;
        let flagged = self.flagged.read().await;
        debug!(
            "Anti-cheat stats: {} tracked players, {} flagged",
            players.len(),
            flagged.len()
        );
    }
}

impl Default for AntiCheat {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate that a position change is reasonable
pub fn validate_movement_delta(
    old_x: u16,
    old_y: u16,
    new_x: u16,
    new_y: u16,
    elapsed_ms: u64,
) -> bool {
    let dx = (new_x as f64 - old_x as f64).abs();
    let dy = (new_y as f64 - old_y as f64).abs();
    let distance = (dx * dx + dy * dy).sqrt();

    // Calculate maximum allowed distance based on time elapsed
    let max_distance = (elapsed_ms as f64 / 1000.0) * MAX_PLAYER_SPEED * 2.0;

    distance <= max_distance.max(50.0) // Always allow at least 50 pixels for lag compensation
}

/// Check if coordinates are within valid room bounds
pub fn validate_position_bounds(x: u16, y: u16) -> bool {
    x <= MAX_ROOM_X && y <= MAX_ROOM_Y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_normal_movement() {
        let ac = AntiCheat::new();
        ac.init_player(1, 100, 100, 1).await;

        // Small movement should be clean
        let result = ac.check_movement(1, 110, 100, 1).await;
        assert!(result.is_clean());
    }

    #[tokio::test]
    async fn test_teleport_detection() {
        let ac = AntiCheat::new();
        ac.init_player(1, 100, 100, 1).await;

        // Large instant movement should be suspicious
        let result = ac.check_movement(1, 5000, 100, 1).await;
        assert!(!result.is_clean());
    }

    #[tokio::test]
    async fn test_legitimate_warp() {
        let ac = AntiCheat::new();
        ac.init_player(1, 100, 100, 1).await;

        // Mark warp as allowed
        ac.allow_warp(1).await;

        // Large movement after warp should be clean
        let result = ac.check_movement(1, 5000, 100, 1).await;
        assert!(result.is_clean());
    }

    #[tokio::test]
    async fn test_room_change() {
        let ac = AntiCheat::new();
        ac.init_player(1, 100, 100, 1).await;

        // Room change resets position tracking
        let result = ac.check_movement(1, 500, 200, 2).await;
        assert!(result.is_clean());
    }

    #[test]
    fn test_movement_validation() {
        // Normal movement
        assert!(validate_movement_delta(100, 100, 110, 100, 100));

        // Too fast
        assert!(!validate_movement_delta(100, 100, 1000, 100, 10));

        // Long time = more distance allowed
        assert!(validate_movement_delta(100, 100, 600, 100, 2000));
    }

    #[test]
    fn test_position_bounds() {
        assert!(validate_position_bounds(100, 100));
        assert!(validate_position_bounds(3000, 1000));
        assert!(!validate_position_bounds(60000, 100));
    }
}
