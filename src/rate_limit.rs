//! Rate limiting module for anti-spam and flood protection
//!
//! Implements per-player, per-action rate limiting to prevent:
//! - Chat spam
//! - Movement flooding
//! - Action spam (item use, shop purchases)
//! - Login brute-forcing

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Rate limit configuration for a specific action type
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of actions allowed in the window
    pub max_actions: u32,
    /// Time window for the rate limit
    pub window: Duration,
    /// Cooldown after exceeding the limit
    pub cooldown: Duration,
    /// Whether to log violations
    pub log_violations: bool,
}

impl RateLimitConfig {
    pub fn new(max_actions: u32, window_secs: u64, cooldown_secs: u64) -> Self {
        Self {
            max_actions,
            window: Duration::from_secs(window_secs),
            cooldown: Duration::from_secs(cooldown_secs),
            log_violations: true,
        }
    }
}

/// Action types that can be rate limited
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionType {
    /// Chat messages
    Chat,
    /// Movement updates
    Movement,
    /// Item usage
    UseItem,
    /// Shop purchases
    ShopBuy,
    /// Bank transactions
    Bank,
    /// Login attempts (per IP)
    Login,
    /// Registration attempts (per IP)
    Register,
    /// Warp requests
    Warp,
    /// Mail sending
    Mail,
    /// BBS posting
    BbsPost,
    /// Generic action (fallback)
    Generic,
}

impl ActionType {
    /// Get the default rate limit config for this action type
    pub fn default_config(&self) -> RateLimitConfig {
        match self {
            // Chat: 10 messages per 10 seconds, 5 second cooldown
            ActionType::Chat => RateLimitConfig::new(10, 10, 5),
            
            // Movement: 60 updates per second (client runs at 30fps, allow some margin)
            // This is very lenient to avoid false positives
            ActionType::Movement => RateLimitConfig::new(120, 1, 1),
            
            // Item use: 5 per 10 seconds
            ActionType::UseItem => RateLimitConfig::new(5, 10, 3),
            
            // Shop: 10 purchases per minute
            ActionType::ShopBuy => RateLimitConfig::new(10, 60, 5),
            
            // Bank: 20 transactions per minute
            ActionType::Bank => RateLimitConfig::new(20, 60, 5),
            
            // Login: 5 attempts per minute per IP
            ActionType::Login => RateLimitConfig::new(5, 60, 30),
            
            // Register: 3 attempts per 5 minutes per IP
            ActionType::Register => RateLimitConfig::new(3, 300, 60),
            
            // Warp: 5 per 30 seconds
            ActionType::Warp => RateLimitConfig::new(5, 30, 5),
            
            // Mail: 10 per minute
            ActionType::Mail => RateLimitConfig::new(10, 60, 10),
            
            // BBS: 5 posts per 5 minutes
            ActionType::BbsPost => RateLimitConfig::new(5, 300, 30),
            
            // Generic: 30 per minute
            ActionType::Generic => RateLimitConfig::new(30, 60, 5),
        }
    }
}

/// Tracks rate limit state for a single action type
#[derive(Debug, Clone)]
struct RateLimitBucket {
    /// Timestamps of recent actions
    actions: Vec<Instant>,
    /// When the cooldown ends (if in cooldown)
    cooldown_until: Option<Instant>,
    /// Config for this bucket
    config: RateLimitConfig,
}

impl RateLimitBucket {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            actions: Vec::with_capacity(config.max_actions as usize),
            cooldown_until: None,
            config,
        }
    }

    /// Check if action is allowed, and record it if so
    fn check_and_record(&mut self) -> RateLimitResult {
        let now = Instant::now();

        // Check if in cooldown
        if let Some(until) = self.cooldown_until {
            if now < until {
                let remaining = until - now;
                return RateLimitResult::InCooldown { remaining };
            }
            // Cooldown expired, clear it
            self.cooldown_until = None;
        }

        // Remove old actions outside the window
        let window_start = now - self.config.window;
        self.actions.retain(|&t| t > window_start);

        // Check if under limit
        if self.actions.len() < self.config.max_actions as usize {
            self.actions.push(now);
            RateLimitResult::Allowed
        } else {
            // Rate limit exceeded, enter cooldown
            self.cooldown_until = Some(now + self.config.cooldown);
            RateLimitResult::Exceeded {
                cooldown: self.config.cooldown,
            }
        }
    }

    /// Get current usage info
    fn usage(&self) -> (u32, u32) {
        let now = Instant::now();
        let window_start = now - self.config.window;
        let current = self.actions.iter().filter(|&&t| t > window_start).count() as u32;
        (current, self.config.max_actions)
    }
}

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub enum RateLimitResult {
    /// Action is allowed
    Allowed,
    /// Rate limit exceeded, player entered cooldown
    Exceeded { cooldown: Duration },
    /// Player is in cooldown from previous violation
    InCooldown { remaining: Duration },
}

impl RateLimitResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed)
    }
}

/// Per-player rate limiter
#[derive(Debug)]
struct PlayerRateLimiter {
    /// Rate limit buckets per action type
    buckets: HashMap<ActionType, RateLimitBucket>,
    /// Total violations for this player
    total_violations: u32,
    /// Last activity time
    last_activity: Instant,
}

impl PlayerRateLimiter {
    fn new() -> Self {
        Self {
            buckets: HashMap::new(),
            total_violations: 0,
            last_activity: Instant::now(),
        }
    }

    fn check(&mut self, action: ActionType) -> RateLimitResult {
        self.last_activity = Instant::now();

        let bucket = self.buckets.entry(action).or_insert_with(|| {
            RateLimitBucket::new(action.default_config())
        });

        let result = bucket.check_and_record();

        if !result.is_allowed() {
            self.total_violations += 1;
        }

        result
    }

    fn is_stale(&self, timeout: Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }
}

/// Global rate limiter managing all players
pub struct RateLimiter {
    /// Per-player rate limiters (keyed by session ID)
    players: RwLock<HashMap<u64, PlayerRateLimiter>>,
    /// Per-IP rate limiters for unauthenticated actions
    ips: RwLock<HashMap<String, PlayerRateLimiter>>,
    /// Custom configs per action type (overrides defaults)
    configs: RwLock<HashMap<ActionType, RateLimitConfig>>,
    /// Cleanup interval
    cleanup_interval: Duration,
    /// Stale entry timeout
    stale_timeout: Duration,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            players: RwLock::new(HashMap::new()),
            ips: RwLock::new(HashMap::new()),
            configs: RwLock::new(HashMap::new()),
            cleanup_interval: Duration::from_secs(60),
            stale_timeout: Duration::from_secs(300),
        }
    }

    /// Check if a player action is rate limited
    pub async fn check_player(&self, session_id: u64, action: ActionType) -> RateLimitResult {
        let mut players = self.players.write().await;
        let limiter = players.entry(session_id).or_insert_with(PlayerRateLimiter::new);
        let result = limiter.check(action);

        if !result.is_allowed() {
            if limiter.buckets.get(&action).map(|b| b.config.log_violations).unwrap_or(true) {
                warn!(
                    "Rate limit {:?} for session {}: {:?} (total violations: {})",
                    action, session_id, result, limiter.total_violations
                );
            }
        }

        result
    }

    /// Check if an IP action is rate limited (for unauthenticated requests)
    pub async fn check_ip(&self, ip: &str, action: ActionType) -> RateLimitResult {
        let mut ips = self.ips.write().await;
        let limiter = ips.entry(ip.to_string()).or_insert_with(PlayerRateLimiter::new);
        let result = limiter.check(action);

        if !result.is_allowed() {
            warn!(
                "Rate limit {:?} for IP {}: {:?}",
                action, ip, result
            );
        }

        result
    }

    /// Get violation count for a player
    pub async fn get_violations(&self, session_id: u64) -> u32 {
        let players = self.players.read().await;
        players.get(&session_id).map(|l| l.total_violations).unwrap_or(0)
    }

    /// Check if player should be warned/kicked for too many violations
    pub async fn should_warn(&self, session_id: u64) -> bool {
        self.get_violations(session_id).await >= 10
    }

    /// Check if player should be kicked for excessive violations
    pub async fn should_kick(&self, session_id: u64) -> bool {
        self.get_violations(session_id).await >= 50
    }

    /// Check if player should be temp-banned for severe violations
    pub async fn should_temp_ban(&self, session_id: u64) -> bool {
        self.get_violations(session_id).await >= 100
    }

    /// Remove a player's rate limit state (on disconnect)
    pub async fn remove_player(&self, session_id: u64) {
        let mut players = self.players.write().await;
        players.remove(&session_id);
        debug!("Removed rate limit state for session {}", session_id);
    }

    /// Clean up stale entries
    pub async fn cleanup(&self) {
        let mut players = self.players.write().await;
        let before = players.len();
        players.retain(|_, l| !l.is_stale(self.stale_timeout));
        let removed = before - players.len();
        if removed > 0 {
            debug!("Cleaned up {} stale player rate limiters", removed);
        }
        drop(players);

        let mut ips = self.ips.write().await;
        let before = ips.len();
        ips.retain(|_, l| !l.is_stale(self.stale_timeout));
        let removed = before - ips.len();
        if removed > 0 {
            debug!("Cleaned up {} stale IP rate limiters", removed);
        }
    }

    /// Override config for a specific action type
    pub async fn set_config(&self, action: ActionType, config: RateLimitConfig) {
        let mut configs = self.configs.write().await;
        configs.insert(action, config);
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper macro for rate limiting in handlers
#[macro_export]
macro_rules! rate_limit {
    ($rate_limiter:expr, $session_id:expr, $action:expr) => {{
        use $crate::rate_limit::{ActionType, RateLimitResult};
        
        let result = $rate_limiter.check_player($session_id, $action).await;
        if !result.is_allowed() {
            return Ok(vec![]);
        }
    }};
}

/// Helper to create rate limiter with custom cleanup task
pub fn spawn_rate_limit_cleanup(rate_limiter: Arc<RateLimiter>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            rate_limiter.cleanup().await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_allowed() {
        let limiter = RateLimiter::new();
        
        // First few requests should be allowed
        for _ in 0..5 {
            let result = limiter.check_player(1, ActionType::Chat).await;
            assert!(result.is_allowed());
        }
    }

    #[tokio::test]
    async fn test_rate_limit_exceeded() {
        let limiter = RateLimiter::new();
        
        // Exhaust the limit (chat is 10 per 10 seconds)
        for i in 0..12 {
            let result = limiter.check_player(1, ActionType::Chat).await;
            if i < 10 {
                assert!(result.is_allowed(), "Request {} should be allowed", i);
            } else {
                assert!(!result.is_allowed(), "Request {} should be denied", i);
            }
        }
    }

    #[tokio::test]
    async fn test_separate_players() {
        let limiter = RateLimiter::new();
        
        // Each player has their own limits
        for _ in 0..5 {
            assert!(limiter.check_player(1, ActionType::Chat).await.is_allowed());
            assert!(limiter.check_player(2, ActionType::Chat).await.is_allowed());
        }
    }

    #[tokio::test]
    async fn test_ip_rate_limit() {
        let limiter = RateLimiter::new();
        
        // Login is 5 per minute
        for i in 0..7 {
            let result = limiter.check_ip("192.168.1.1", ActionType::Login).await;
            if i < 5 {
                assert!(result.is_allowed());
            } else {
                assert!(!result.is_allowed());
            }
        }
    }
}
