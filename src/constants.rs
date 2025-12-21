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
// GAME LIMITS
// =============================================================================

// Player Limits
pub const MAX_USERNAME_LENGTH: usize = 20;
pub const MIN_USERNAME_LENGTH: usize = 3;
pub const MAX_PASSWORD_LENGTH: usize = 50;
pub const MIN_PASSWORD_LENGTH: usize = 6;
pub const MAX_CHAT_LENGTH: usize = 100;

// Currency
pub const MAX_POINTS: u32 = 10_000_000;
pub const MAX_BANK_BALANCE: u32 = 100_000_000;

// Inventory
pub const INVENTORY_SLOTS: usize = 9;
pub const EMOTE_SLOTS: usize = 5;
pub const OUTFIT_SLOTS: usize = 9;
pub const ACCESSORY_SLOTS: usize = 9;
pub const ITEM_SLOTS: usize = 9;
pub const TOOL_SLOTS: usize = 9;

// Clan
pub const MAX_CLAN_NAME_LENGTH: usize = 20;
pub const MIN_CLAN_NAME_LENGTH: usize = 3;
pub const MAX_CLAN_MEMBERS: usize = 50;
pub const MAX_CLAN_DESCRIPTION: usize = 500;
pub const CLAN_CREATION_COST: u32 = 10_000;

// Mail
pub const MAX_MAIL_SUBJECT: usize = 50;
pub const MAX_MAIL_BODY: usize = 1000;
pub const MAX_MAILS_PER_PAGE: usize = 5;

// BBS
pub const MAX_BBS_TITLE: usize = 100;
pub const MAX_BBS_CONTENT: usize = 5000;
pub const MAX_BBS_POSTS_PER_PAGE: usize = 10;

// Room
pub const MAX_ROOM_ID: u16 = 1000;
pub const MAX_PLAYERS_PER_ROOM: usize = 50;

// Network
pub const MAX_MESSAGE_SIZE: usize = 8192;
pub const CONNECTION_TIMEOUT_SECS: u64 = 300;
pub const UNAUTHENTICATED_TIMEOUT_SECS: u64 = 30;

// Connection Limits
pub const MAX_CONNECTIONS_PER_IP: usize = 3;
pub const MAX_TOTAL_CONNECTIONS: usize = 1000;

// =============================================================================
// GAME TIMERS (from original server)
// =============================================================================

/// Collectibles respawn check interval in seconds
pub const COLLECTIBLE_RESPAWN_CHECK_SECS: u64 = 30;

// =============================================================================
// PHYSICS CONSTANTS
// =============================================================================

pub const GRAVITY: f32 = 0.2145; // 0.33 * 0.65
pub const MAX_HORIZONTAL_SPEED: f32 = 1.2;
pub const MAX_VERTICAL_SPEED: f32 = 9.0;
pub const JUMP_SPEED: f32 = -5.3;
pub const NORMAL_FRICTION: f32 = 0.165; // 0.5 * 0.33
pub const ICE_FRICTION: f32 = 0.0066; // 0.02 * 0.33
pub const WALK_ACCELERATION: f32 = 0.33;
pub const WATER_GRAVITY_REDUCTION: f32 = 0.5;
pub const WATER_MAX_VERTICAL_SPEED: f32 = 0.75; // 1.5 * 0.5

// =============================================================================
// RESPONSE CODES
// =============================================================================

/// Login response codes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginResult {
    Success = 1,
    NoAccount = 2,
    WrongPassword = 3,
    AlreadyLoggedIn = 4,
    VersionMismatch = 5,
    AccountBanned = 6,
    IpBanned1 = 7,
    IpBanned2 = 8,
}

/// Register response codes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterResult {
    Success = 1,
    AccountExists = 2,
    IpBanned = 3,
    MacBanned = 4,
}

/// Bank operation response codes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankResult {
    DepositOk = 1,
    WithdrawOk = 2,
    TransferOk = 3,
    ReceiverNotFound = 4,
}

/// Shop buy failure codes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopBuyResult {
    InsufficientFunds = 1,
    InventoryFull = 2,
    OutOfStock = 3,
}

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

pub const TICKS_PER_SECOND: u64 = 60;
pub const MS_PER_TICK: u64 = 16;
pub const SAVE_INTERVAL_SECS: u64 = 300;
pub const CLEANUP_INTERVAL_SECS: u64 = 60;

// Day of week constants
pub const SUNDAY: u8 = 1;
pub const MONDAY: u8 = 2;
pub const TUESDAY: u8 = 3;
pub const WEDNESDAY: u8 = 4;
pub const THURSDAY: u8 = 5;
pub const FRIDAY: u8 = 6;
pub const SATURDAY: u8 = 7;

/// Day of week (matches client expectations)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayOfWeek {
    Sunday = 1,
    Monday = 2,
    Tuesday = 3,
    Wednesday = 4,
    Thursday = 5,
    Friday = 6,
    Saturday = 7,
}

// =============================================================================
// DEFAULT PLAYER VALUES
// =============================================================================

// Note: These defaults should match config/game.toml [defaults] section.
// The canonical values are in the config file; these are fallbacks for
// PlayerSession initialization before config is loaded.

/// Default spawn X coordinate (matches game.toml defaults.spawn_x)
pub const DEFAULT_SPAWN_X: u16 = 385;
/// Default spawn Y coordinate (matches game.toml defaults.spawn_y)
pub const DEFAULT_SPAWN_Y: u16 = 71;
/// Default spawn room ID (matches game.toml defaults.spawn_room)
/// Room 32 is the correct main spawn area.
pub const DEFAULT_SPAWN_ROOM: u16 = 32;
pub const DEFAULT_BODY_ID: u16 = 1;
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

// =============================================================================
// RATE LIMITING CONSTANTS
// =============================================================================

/// Chat: max messages per window
pub const RATE_LIMIT_CHAT_MAX: u32 = 10;
/// Chat: time window (seconds)
pub const RATE_LIMIT_CHAT_WINDOW_SECS: u64 = 10;
/// Chat: cooldown after exceeding limit (seconds)
pub const RATE_LIMIT_CHAT_COOLDOWN_SECS: u64 = 5;

/// Movement: max updates per window (very lenient for client at 30fps)
pub const RATE_LIMIT_MOVEMENT_MAX: u32 = 120;
/// Movement: time window (seconds)
pub const RATE_LIMIT_MOVEMENT_WINDOW_SECS: u64 = 1;

/// Item use: max per window
pub const RATE_LIMIT_ITEM_USE_MAX: u32 = 5;
/// Item use: time window (seconds)
pub const RATE_LIMIT_ITEM_USE_WINDOW_SECS: u64 = 10;

/// Shop: max purchases per window
pub const RATE_LIMIT_SHOP_MAX: u32 = 10;
/// Shop: time window (seconds)
pub const RATE_LIMIT_SHOP_WINDOW_SECS: u64 = 60;

/// Login attempts: max per window (per IP)
pub const RATE_LIMIT_LOGIN_MAX: u32 = 5;
/// Login: time window (seconds)
pub const RATE_LIMIT_LOGIN_WINDOW_SECS: u64 = 60;
/// Login: cooldown after exceeding limit (seconds)
pub const RATE_LIMIT_LOGIN_COOLDOWN_SECS: u64 = 30;

/// Register attempts: max per window (per IP)
pub const RATE_LIMIT_REGISTER_MAX: u32 = 3;
/// Register: time window (seconds)
pub const RATE_LIMIT_REGISTER_WINDOW_SECS: u64 = 300;

// =============================================================================
// VALIDATION CONSTANTS
// =============================================================================

/// Maximum valid item ID (based on db_items.gml)
pub const MAX_ITEM_ID: u16 = 61;

/// Maximum valid emote ID
pub const MAX_EMOTE_ID: u8 = 20;

/// Maximum valid action ID (sit, etc.)
pub const MAX_ACTION_ID: u8 = 10;

/// Maximum valid direction code for movement
pub const MAX_DIRECTION_CODE: u8 = 13;

// =============================================================================
// ROOM IDS
// =============================================================================

/// Room IDs matching GameMaker project room indices
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomId {
    // Menu/System rooms (0-8)
    Load = 0,
    Menu = 1,
    Intro = 2,
    NewAcc = 3,
    Login = 4,
    ServList = 5,
    Credits = 6,
    Options = 7,
    SetControls = 8,
    // Main world rooms
    AroundNew1 = 37,
    AroundNew2 = 38,
    AroundNew3 = 39,
    AroundNew4 = 40,
    AroundNew5 = 41,
}

impl RoomId {
    pub fn is_menu_room(room_id: u16) -> bool {
        room_id <= 8
    }

    pub fn is_playable_room(room_id: u16) -> bool {
        room_id > 8
    }
}
