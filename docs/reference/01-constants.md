# Game Constants Reference

## Message Type Constants

All 141 message types from the client:

```rust
// Authentication (2)
pub const MSG_REGISTER: u16 = 7;
pub const MSG_LOGIN: u16 = 10;

// Player Management (6)
pub const MSG_NEW_PLAYER: u16 = 1;
pub const MSG_MOVE_PLAYER: u16 = 2;
pub const MSG_CHANGE_ROOM: u16 = 5;
pub const MSG_LOGOUT: u16 = 6;
pub const MSG_PLAYER_LEAVE: u16 = 11;

// Movement & Control (5)
pub const MSG_MOVE_GET_ON: u16 = 124;
pub const MSG_MOVE_GET_OFF: u16 = 125;
pub const MSG_PLAYER_STOP: u16 = 43;
pub const MSG_CANMOVE_TRUE: u16 = 42;
pub const MSG_POSITION: u16 = 15;

// Actions (4)
pub const MSG_ACTION: u16 = 12;
pub const MSG_CHANGE_OUT: u16 = 13;
pub const MSG_CHANGE_ACS1: u16 = 25;
pub const MSG_CHANGE_ACS2: u16 = 26;

// Warping (2)
pub const MSG_WARP: u16 = 14;
pub const MSG_CREATE: u16 = 16;

// Communication (3)
pub const MSG_CHAT: u16 = 17;
pub const MSG_EMOTE: u16 = 23;
pub const MSG_EMOTE_DICE: u16 = 93;
pub const MSG_PLAYER_TYPING: u16 = 133;

// Server Utility (6)
pub const MSG_PING: u16 = 9;
pub const MSG_TIME: u16 = 21;
pub const MSG_TIME_UPDATE: u16 = 74;
pub const MSG_MUSIC_CHANGE: u16 = 22;
pub const MSG_SERVER_CLOSE: u16 = 24;
pub const MSG_PING_REQ: u16 = 117;
pub const MSG_SERVER_TIME: u16 = 118;
pub const MSG_SERVER_TIME_RESET: u16 = 119;

// Items & Inventory (11)
pub const MSG_USE_ITEM: u16 = 31;
pub const MSG_GET_ITEM: u16 = 41;
pub const MSG_DISCARD_ITEM: u16 = 39;
pub const MSG_DISCARDED_ITEM_TAKE: u16 = 40;
pub const MSG_RETURN_ITEM: u16 = 50;
pub const MSG_ITEM_MAP_SET: u16 = 48;
pub const MSG_ITEM_MAP_SET_DESTROY: u16 = 49;

// Collectibles (4)
pub const MSG_COLLECTIBLE_INFO: u16 = 32;
pub const MSG_COLLECTIBLE_TAKE_SELF: u16 = 33;
pub const MSG_COLLECTIBLE_TAKEN: u16 = 34;
pub const MSG_COLLECTIBLE_EVOLVE: u16 = 132;

// One-Time Items (4)
pub const MSG_ONE_TIME_INFO: u16 = 35;
pub const MSG_ONE_TIME_DISAPPEAR: u16 = 36;
pub const MSG_ONE_TIME_GET: u16 = 37;

// Shop & Economy (7)
pub const MSG_ROOM_SHOP_INFO: u16 = 27;
pub const MSG_SHOP_BUY: u16 = 28;
pub const MSG_SHOP_BUY_FAIL: u16 = 29;
pub const MSG_SHOP_STOCK: u16 = 30;
pub const MSG_SELL_REQ_PRICES: u16 = 53;
pub const MSG_SELL: u16 = 54;

// Quest System (10)
pub const MSG_QUEST_BEGIN: u16 = 83;
pub const MSG_QUEST_CLEAR: u16 = 84;
pub const MSG_QUEST_STEP_INC: u16 = 85;
pub const MSG_QUEST_CANCEL: u16 = 86;
pub const MSG_QUEST_NPC_REQ: u16 = 87;
pub const MSG_QUEST_VAR_CHECK: u16 = 88;
pub const MSG_QUEST_VAR_INC: u16 = 89;
pub const MSG_QUEST_VAR_SET: u16 = 90;
pub const MSG_QUEST_STATUS_REQ: u16 = 91;
pub const MSG_QUEST_REWARD: u16 = 92;

// Clan System (7)
pub const MSG_CLAN_CREATE: u16 = 126;
pub const MSG_CLAN_DISSOLVE: u16 = 127;
pub const MSG_CLAN_INVITE: u16 = 128;
pub const MSG_CLAN_LEAVE: u16 = 129;
pub const MSG_CLAN_INFO: u16 = 130;
pub const MSG_CLAN_ADMIN: u16 = 131;

// Banking & Storage (4)
pub const MSG_BANK_PROCESS: u16 = 45;
pub const MSG_STORAGE_REQ: u16 = 56;
pub const MSG_STORAGE_PAGES: u16 = 57;
pub const MSG_STORAGE_MOVE: u16 = 58;

// Mail System (4)
pub const MSG_MAILBOX: u16 = 47;
pub const MSG_MAIL_SEND: u16 = 78;
pub const MSG_MAILPAPER_REQ: u16 = 79;
pub const MSG_MAIL_RECEIVER_CHECK: u16 = 80;

// Bulletin Board (8)
pub const MSG_BBS_REQUEST_CATEGORIES: u16 = 134;
pub const MSG_BBS_REQUEST_GUI: u16 = 135;
pub const MSG_BBS_REQUEST_MAX_PAGES: u16 = 136;
pub const MSG_BBS_REQUEST_MESSAGES: u16 = 137;
pub const MSG_BBS_REQUEST_MESSAGE_CONTENT: u16 = 138;
pub const MSG_BBS_REPORT_MESSAGE: u16 = 139;
pub const MSG_BBS_REQUEST_POST: u16 = 140;
pub const MSG_BBS_POST: u16 = 141;

// Planting System (11)
pub const MSG_PLANT_SPOT_FREE: u16 = 63;
pub const MSG_PLANT_SPOT_USED: u16 = 64;
pub const MSG_PLANT_DIE: u16 = 65;
pub const MSG_PLANT_GROW: u16 = 66;
pub const MSG_PLANT_ADD_PINWHEEL: u16 = 67;
pub const MSG_PLANT_ADD_FAIRY: u16 = 68;
pub const MSG_PLANT_GET_FRUIT: u16 = 69;
pub const MSG_PLANT_HAS_FRUIT: u16 = 70;
pub const MSG_TREE_PLANTED_INC: u16 = 94;

// Building System (5)
pub const MSG_BUILD_SPOT_FREE: u16 = 103;
pub const MSG_BUILD_SPOT_USED: u16 = 104;
pub const MSG_BUILD_SPOT_BECOME_FREE: u16 = 105;
pub const MSG_OBJECTS_BUILT_INC: u16 = 106;

// Cannon System (4)
pub const MSG_CANNON_ENTER: u16 = 98;
pub const MSG_CANNON_MOVE: u16 = 99;
pub const MSG_CANNON_SET_POWER: u16 = 100;
pub const MSG_CANNON_SHOOT: u16 = 101;

// Racing System (4)
pub const MSG_RACE_INFO: u16 = 120;
pub const MSG_RACE_START: u16 = 121;
pub const MSG_RACE_CHECKPOINT: u16 = 122;
pub const MSG_RACE_END: u16 = 123;

// Misc (remaining)
pub const MSG_POINT: u16 = 18;
pub const MSG_SAVE: u16 = 19;
pub const MSG_REQUEST_STATUS: u16 = 44;
pub const MSG_MOD_ACTION: u16 = 46;
pub const MSG_PLAYER_SET_STATUS: u16 = 48;
pub const MSG_SIGN_TXT_REQUEST: u16 = 52;
pub const MSG_POINTS_DEC: u16 = 53;
pub const MSG_GET_TOP_POINTS: u16 = 73;
pub const MSG_GET_SOMETHING: u16 = 75;
pub const MSG_GET_WARP_INFO: u16 = 76;
pub const MSG_WARP_CENTER_USE_SLOT: u16 = 77;
pub const MSG_TOOL_EQUIP: u16 = 81;
pub const MSG_TOOL_UNEQUIP: u16 = 82;
pub const MSG_MUSIC_CHANGER_LIST: u16 = 95;
pub const MSG_MUSIC_CHANGER_SET: u16 = 96;
pub const MSG_PLAYER_THROW: u16 = 97;
pub const MSG_UPGRADER_GET: u16 = 108;
pub const MSG_UPGRADER_POINTS: u16 = 109;
pub const MSG_UPGRADER_INVEST: u16 = 110;
pub const MSG_UPGRADER_APPEAR: u16 = 111;
pub const MSG_UNLOCKABLE_EXISTS: u16 = 112;
pub const MSG_BUY_GUM: u16 = 113;
pub const MSG_BUY_SODA: u16 = 114;
pub const MSG_SWITCH_SET: u16 = 116;

// Reserved/Unused
pub const MSG_PLAYER_EXIST: u16 = 3;    // Unused
pub const MSG_SENDID: u16 = 4;          // Unused
pub const MSG_SEND_STATUS: u16 = 8;     // Unused
```

## Movement Direction Codes

```rust
pub const DIR_START_LEFT_GROUND: u8 = 1;
pub const DIR_START_RIGHT_GROUND: u8 = 2;
pub const DIR_JUMP: u8 = 3;
pub const DIR_DUCK: u8 = 4;
pub const DIR_STOP_LEFT_GROUND: u8 = 5;
pub const DIR_STOP_RIGHT_GROUND: u8 = 6;
pub const DIR_RELEASE_JUMP: u8 = 7;
pub const DIR_RELEASE_DUCK: u8 = 8;
pub const DIR_LANDING: u8 = 9;
pub const DIR_START_LEFT_AIR: u8 = 10;
pub const DIR_START_RIGHT_AIR: u8 = 11;
pub const DIR_STOP_LEFT_AIR: u8 = 12;
pub const DIR_STOP_RIGHT_AIR: u8 = 13;
```

## Game Limits

```rust
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
pub const PING_INTERVAL_SECS: u64 = 30;
```

## Physics Constants

```rust
pub const GRAVITY: f32 = 0.2145;  // 0.33 * 0.65
pub const MAX_HORIZONTAL_SPEED: f32 = 1.2;
pub const MAX_VERTICAL_SPEED: f32 = 9.0;
pub const JUMP_SPEED: f32 = -5.3;
pub const NORMAL_FRICTION: f32 = 0.165;  // 0.5 * 0.33
pub const ICE_FRICTION: f32 = 0.0066;    // 0.02 * 0.33
pub const WALK_ACCELERATION: f32 = 0.33;
pub const WATER_GRAVITY_REDUCTION: f32 = 0.5;
pub const WATER_MAX_VERTICAL_SPEED: f32 = 0.75;  // 1.5 * 0.5
```

## Item ID Categories

```rust
// Consumables (1-38)
pub const ITEM_WARP_WING: u16 = 1;
pub const ITEM_SMOKEBOMB: u16 = 2;
pub const ITEM_APPLEBOMB: u16 = 3;
pub const ITEM_BUBBLES: u16 = 4;
pub const ITEM_SLIMEBAG_50: u16 = 5;
pub const ITEM_SLIMEBAG_200: u16 = 6;
pub const ITEM_SLIMEBAG_500: u16 = 7;
pub const ITEM_CHICKEN_MINE: u16 = 8;
pub const ITEM_SIMPLE_SEED: u16 = 9;
pub const ITEM_FAIRY: u16 = 10;
pub const ITEM_PINWHEEL_BLUE: u16 = 11;
pub const ITEM_PINWHEEL_RED: u16 = 12;
pub const ITEM_PINWHEEL_GLOW: u16 = 13;
pub const ITEM_BRIGHT_DRINK: u16 = 23;
pub const ITEM_BLUE_SEED: u16 = 24;
pub const ITEM_JUICY_BANGO: u16 = 25;
pub const ITEM_WEAK_CANNON_KIT: u16 = 26;

// Gums (27-32)
pub const ITEM_RED_GUM: u16 = 27;
pub const ITEM_ORANGE_GUM: u16 = 28;
pub const ITEM_GREEN_GUM: u16 = 29;
pub const ITEM_BLUE_GUM: u16 = 30;
pub const ITEM_PINK_GUM: u16 = 31;
pub const ITEM_WHITE_GUM: u16 = 32;

// Currency Items
pub const ITEM_LUCKY_COIN: u16 = 33;

// Sodas (34-38)
pub const ITEM_BUNNY_SODA: u16 = 34;
pub const ITEM_SLIME_SODA: u16 = 35;
pub const ITEM_PENGUIN_SODA: u16 = 36;
pub const ITEM_SPEED_SODA: u16 = 37;
pub const ITEM_JUMP_SODA: u16 = 38;

// Materials (39-50)
pub const ITEM_SLEENMIUM: u16 = 39;
pub const ITEM_SLEDMIUM: u16 = 40;
pub const ITEM_SLUEMIUM: u16 = 41;
pub const ITEM_SLINKMIUM: u16 = 42;
pub const ITEM_SLELLOYMIUM: u16 = 43;
pub const ITEM_SLAYMIUM: u16 = 44;
pub const ITEM_SLACKMIUM: u16 = 45;

// Quest Items (51-61)
pub const ITEM_PROOF_NATURE: u16 = 51;
pub const ITEM_PROOF_EARTH: u16 = 52;
pub const ITEM_PROOF_WATER: u16 = 53;
pub const ITEM_PROOF_FIRE: u16 = 54;
pub const ITEM_PROOF_STONE: u16 = 55;
pub const ITEM_PROOF_WIND: u16 = 56;
pub const ITEM_BLAZING_BUBBLE: u16 = 57;
pub const ITEM_SQUISHY_MUSHROOM: u16 = 58;
pub const ITEM_STINKY_MUSHROOM: u16 = 59;
pub const ITEM_BELL_TWIG: u16 = 60;
pub const ITEM_IRRLICHT: u16 = 61;
```

## Encryption Keys

```rust
// CRITICAL: These MUST match client hardcoded keys
pub const CLIENT_ENCRYPT_KEY: &[u8] = b"retrtz7jmijb5467n47";
pub const CLIENT_DECRYPT_KEY: &[u8] = b"t54gz65u74njb6zg6";

// Server uses opposite keys:
// - Decrypt incoming: CLIENT_ENCRYPT_KEY
// - Encrypt outgoing: CLIENT_DECRYPT_KEY
```

## Time Constants

```rust
pub const TICKS_PER_SECOND: u64 = 60;
pub const MS_PER_TICK: u64 = 16;  // ~16.67ms
pub const SAVE_INTERVAL_SECS: u64 = 300;  // 5 minutes
pub const CLEANUP_INTERVAL_SECS: u64 = 60;  // 1 minute
```

## Error/Response Codes

```rust
// Login responses
pub const LOGIN_SUCCESS: u8 = 1;
pub const LOGIN_NO_ACCOUNT: u8 = 2;
pub const LOGIN_WRONG_PASSWORD: u8 = 3;
pub const LOGIN_ALREADY_LOGGED_IN: u8 = 4;
pub const LOGIN_VERSION_MISMATCH: u8 = 5;
pub const LOGIN_ACCOUNT_BANNED: u8 = 6;
pub const LOGIN_IP_BANNED_1: u8 = 7;
pub const LOGIN_IP_BANNED_2: u8 = 8;

// Register responses
pub const REGISTER_SUCCESS: u8 = 1;
pub const REGISTER_EXISTS: u8 = 2;
pub const REGISTER_IP_BANNED: u8 = 3;
pub const REGISTER_MAC_BANNED: u8 = 4;

// Bank responses
pub const BANK_DEPOSIT_OK: u8 = 1;
pub const BANK_WITHDRAW_OK: u8 = 2;
pub const BANK_TRANSFER_OK: u8 = 3;
pub const BANK_RECEIVER_NOT_FOUND: u8 = 4;

// Shop responses
pub const SHOP_BUY_INSUFFICIENT_FUNDS: u8 = 1;
pub const SHOP_BUY_INV_FULL: u8 = 2;
pub const SHOP_BUY_OUT_OF_STOCK: u8 = 3;
```

## Day of Week

```rust
pub const SUNDAY: u8 = 1;
pub const MONDAY: u8 = 2;
pub const TUESDAY: u8 = 3;
pub const WEDNESDAY: u8 = 4;
pub const THURSDAY: u8 = 5;
pub const FRIDAY: u8 = 6;
pub const SATURDAY: u8 = 7;
```

---

**Usage:** Import these constants in your Rust code for type-safe message handling.
