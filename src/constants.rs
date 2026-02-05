//! Game constants for Slime Online 2 server
//!
//! All values are based on the v0.106 client and cannot be changed.
//!
//! Note: Message types are defined in `protocol::MessageType` enum, not here.

// =============================================================================
// ENCRYPTION KEYS - CRITICAL: Must match client hardcoded keys
// =============================================================================

/// Key the client uses to encrypt messages (server decrypts with this)
pub const CLIENT_ENCRYPT_KEY: &[u8] = b"retrtz7jmijb5467n47";
/// Key the client uses to decrypt messages (server encrypts with this)
pub const CLIENT_DECRYPT_KEY: &[u8] = b"t54gz65u74njb6zg6";

// =============================================================================
// SERVER CONFIGURATION
// =============================================================================

pub const DEFAULT_PORT: u16 = 5555;
pub const PROTOCOL_VERSION: &str = "0.106";

// =============================================================================
// MOVEMENT DIRECTION CODES
// =============================================================================

/// Direction codes sent with MSG_MOVE_PLAYER
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    StartLeftGround = 1,
    StartRightGround = 2,
    Jump = 3,
    Duck = 4,
    StopLeftGround = 5,
    StopRightGround = 6,
    ReleaseJump = 7,
    ReleaseDuck = 8,
    Landing = 9,
    StartLeftAir = 10,
    StartRightAir = 11,
    StopLeftAir = 12,
    StopRightAir = 13,
}

impl Direction {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::StartLeftGround),
            2 => Some(Self::StartRightGround),
            3 => Some(Self::Jump),
            4 => Some(Self::Duck),
            5 => Some(Self::StopLeftGround),
            6 => Some(Self::StopRightGround),
            7 => Some(Self::ReleaseJump),
            8 => Some(Self::ReleaseDuck),
            9 => Some(Self::Landing),
            10 => Some(Self::StartLeftAir),
            11 => Some(Self::StartRightAir),
            12 => Some(Self::StopLeftAir),
            13 => Some(Self::StopRightAir),
            _ => None,
        }
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StartLeftGround => write!(f, "StartLeftGround"),
            Self::StartRightGround => write!(f, "StartRightGround"),
            Self::Jump => write!(f, "Jump"),
            Self::Duck => write!(f, "Duck"),
            Self::StopLeftGround => write!(f, "StopLeftGround"),
            Self::StopRightGround => write!(f, "StopRightGround"),
            Self::ReleaseJump => write!(f, "ReleaseJump"),
            Self::ReleaseDuck => write!(f, "ReleaseDuck"),
            Self::Landing => write!(f, "Landing"),
            Self::StartLeftAir => write!(f, "StartLeftAir"),
            Self::StartRightAir => write!(f, "StartRightAir"),
            Self::StopLeftAir => write!(f, "StopLeftAir"),
            Self::StopRightAir => write!(f, "StopRightAir"),
        }
    }
}

// =============================================================================
// GAME LIMITS - Engine constraints (not configurable)
// Configurable limits are in game.toml and clans.toml
// =============================================================================

// Network (engine constraints, also in server.toml for config)
pub const MAX_MESSAGE_SIZE: usize = 8192;
pub const CONNECTION_TIMEOUT_SECS: u64 = 300;
pub const UNAUTHENTICATED_TIMEOUT_SECS: u64 = 30;

// Connection Limits (also in server.toml)
pub const MAX_CONNECTIONS_PER_IP: usize = 3;
pub const MAX_TOTAL_CONNECTIONS: usize = 1000;

// =============================================================================
// RESPONSE CODES
// =============================================================================

// Login response constants
pub const LOGIN_SUCCESS: u8 = 1;
pub const LOGIN_NO_ACCOUNT: u8 = 2;
pub const LOGIN_WRONG_PASSWORD: u8 = 3;
pub const LOGIN_ALREADY_LOGGED_IN: u8 = 4;
pub const LOGIN_VERSION_MISMATCH: u8 = 5;
pub const LOGIN_ACCOUNT_BANNED: u8 = 6;
pub const LOGIN_IP_BANNED_1: u8 = 7;
pub const LOGIN_IP_BANNED_2: u8 = 8;

// Register response constants
pub const REGISTER_SUCCESS: u8 = 1;
pub const REGISTER_EXISTS: u8 = 2;
pub const REGISTER_IP_BANNED: u8 = 3;
pub const REGISTER_MAC_BANNED: u8 = 4;

// =============================================================================
// TIME CONSTANTS
// =============================================================================

pub const SAVE_INTERVAL_SECS: u64 = 300;
pub const CLEANUP_INTERVAL_SECS: u64 = 60;

// Day of week constants (used in auth.rs for login response)
pub const SUNDAY: u8 = 1;
pub const MONDAY: u8 = 2;
pub const TUESDAY: u8 = 3;
pub const WEDNESDAY: u8 = 4;
pub const THURSDAY: u8 = 5;
pub const FRIDAY: u8 = 6;
pub const SATURDAY: u8 = 7;

// =============================================================================
// DEFAULT PLAYER VALUES
// =============================================================================

pub const DEFAULT_POINTS: u32 = 0;

// =============================================================================
// ANTI-CHEAT CONSTANTS
// =============================================================================

/// Maximum distance a player can move per update (pixels)
/// Based on: max speed ~10 pixels/frame * 30fps * 2 seconds for lag tolerance
pub const MAX_MOVEMENT_DISTANCE_PER_UPDATE: f64 = 600.0;

/// Maximum reasonable speed (pixels per second)
/// Based on: hspmax=3 * 30fps = 90, with generous margin for lag
pub const MAX_PLAYER_SPEED: f64 = 300.0;

/// Maximum valid X coordinate in any room
pub const MAX_ROOM_X: u16 = 5000;

/// Maximum valid Y coordinate in any room
pub const MAX_ROOM_Y: u16 = 3000;

/// Number of cheat violations before flagging player
pub const CHEAT_VIOLATION_THRESHOLD: u32 = 5;

/// Time window for counting violations (seconds)
pub const CHEAT_VIOLATION_WINDOW_SECS: u64 = 60;

/// Number of flags before kicking player
pub const CHEAT_FLAGS_TO_KICK: u32 = 3;

/// Number of flags before banning player
pub const CHEAT_FLAGS_TO_BAN: u32 = 10;

