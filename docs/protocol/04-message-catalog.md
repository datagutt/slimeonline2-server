# Protocol: Complete Message Catalog

## Overview

This document catalogs all 141 message types in the Slime Online 2 protocol, organized by functional category. Each message includes its ID, direction, payload structure, and implementation notes.

## Message Categories

- [Authentication](#authentication-messages-7-10) (2 messages)
- [Player Management](#player-management-messages-1-6-11) (6 messages)
- [Movement & Physics](#movement--physics-messages-2-14-43-124-125) (5 messages)
- [Communication](#communication-messages-17-133) (2 messages)
- [Items & Inventory](#items--inventory-messages-31-41) (11 messages)
- [Shop & Economy](#shop--economy-messages-27-30-53-55) (5 messages)
- [Quest System](#quest-system-messages-83-92) (10 messages)
- [Clan System](#clan-system-messages-126-132) (7 messages)
- [Mail System](#mail-system-messages-47-78-80) (4 messages)
- [Bulletin Board](#bulletin-board-messages-134-141) (8 messages)
- [Banking & Storage](#banking--storage-messages-45-56-58) (4 messages)
- [Planting & Farming](#planting--farming-messages-63-72-94) (11 messages)
- [Building System](#building-system-messages-103-107) (5 messages)
- [Special Features](#special-features-messages) (15+ messages)
- [Server Utility](#server-utility-messages-9-21-24-117-118) (5 messages)

---

## Authentication Messages (7, 10)

### MSG_REGISTER (7)

**Direction:** Client → Server  
**Purpose:** Create new account

**Client Sends:**
```rust
struct RegisterRequest {
    msg_type: u16,      // 7
    version: String,    // "0.106"
    username: String,   // 3-20 characters
    password: String,   // Plaintext (will be hashed server-side)
    mac_address: String // Hardware ID
}
```

**Server Responds:**
```rust
struct RegisterResponse {
    msg_type: u16,  // 7
    result: u8,     // 1=success, 2=exists, 3=IP banned, 4=MAC banned
}
```

**Validation:**
- Username: 3-20 alphanumeric + underscore
- Password: 6-50 characters (enforce on server)
- Check IP ban list before processing
- Check MAC ban list before processing
- Hash password with bcrypt before storing

---

### MSG_LOGIN (10)

**Direction:** Client ↔ Server  
**Purpose:** Authenticate and load player data

**Client Sends:**
```rust
struct LoginRequest {
    msg_type: u16,      // 10
    version: String,    // "0.106"
    username: String,
    password: String,   // Plaintext
    mac_address: String
}
```

**Server Responds (Success - case 1):**
```rust
struct LoginSuccess {
    msg_type: u16,      // 10
    case: u8,           // 1
    player_id: u16,
    server_time: u32,   // Unix timestamp
    motd: String,       // Message of the day
    day: u8,            // 1-7 (Sun-Sat)
    hour: u8,           // 0-23
    minute: u8,         // 0-59
    username: String,   // Echo back
    spawn_x: u16,
    spawn_y: u16,
    spawn_room: u16,
    body_id: u16,       // Current outfit
    acs1_id: u16,       // Accessory slot 1
    acs2_id: u16,       // Accessory slot 2
    points: u32,        // Currency (Slime Points)
    signature: u8,      // Has signature permission (0/1)
    quest_id: u16,      // Active quest ID (0 = none)
    quest_step: u8,     // Quest progress step
    trees_planted: u16, // Total trees planted stat
    objects_built: u16, // Total objects built stat
    emotes: [u8; 5],    // 5 emote slot IDs
    outfits: [u16; 9],  // 9 outfit slot IDs
    accessories: [u16; 9], // 9 accessory slot IDs
    items: [u16; 9],    // 9 item slot IDs
    tools: [u8; 9],     // 9 tool slot IDs
}
```

**Server Responds (Failure):**
```rust
struct LoginFailure {
    msg_type: u16,  // 10
    case: u8,       // 2-8 (error codes)
}

// Error codes:
// 2 = Account doesn't exist
// 3 = Wrong password
// 4 = Already logged in
// 5 = Version mismatch
// 6 = Account banned
// 7 = IP banned (code 1)
// 8 = IP banned (code 2)
```

**Implementation Notes:**
- Must be first message after TCP connection
- Verify password with bcrypt comparison
- Check if account already logged in (kick old session)
- Load all player data from database
- Create session token (UUID) for tracking
- Send full player state in response

---

## Player Management Messages (1, 6, 11)

### MSG_NEW_PLAYER (1)

**Direction:** Server → Client, Client → Server  
**Purpose:** Notify about new player joining or provide info

**Server → Client (case 1 - Need response):**
```rust
struct NewPlayerJoined {
    msg_type: u16,  // 1
    case: u8,       // 1
    x: u16,
    y: u16,
    player_id: u16,
    room_id: u16,
    username: String,
    body_id: u16,
    acs1_id: u16,
    acs2_id: u16,
}
```

**Client → Server (Response):**
```rust
struct NewPlayerResponse {
    msg_type: u16,  // 1
    target_pid: u16, // Player ID we're responding about
    our_x: u16,      // Our position
    our_y: u16,      // Or 65536 if in cannon
}
```

**Server → Client (case 2 - Just info):**
```rust
struct NewPlayerInfo {
    msg_type: u16,  // 1
    case: u8,       // 2
    x: u16,
    y: u16,
    player_id: u16,
    ileft: u8,      // Movement state (0/1)
    iright: u8,
    iup: u8,
    idown: u8,
    iup_press: u8,
    room_id: u16,
    username: String,
    body_id: u16,
    acs1_id: u16,
    acs2_id: u16,
}
```

**When Sent:**
- Case 1: When NEW player joins server, sent to ALL existing players
- Case 2: When existing player changes rooms, sent to players in new room

---

### MSG_LOGOUT (6) / MSG_PLAYER_LEAVE (6)

**Direction:** Client → Server, Server → Broadcast  
**Purpose:** Player disconnect

**Client → Server:**
```rust
struct LogoutRequest {
    msg_type: u16,  // 6
}
```

**Server → Broadcast:**
```rust
struct PlayerLeft {
    msg_type: u16,  // 6
    player_id: u16,
}
```

**Implementation:**
- Save all player data to database
- Broadcast to all players in same room
- Remove from active sessions
- Close TCP connection gracefully

---

## Movement & Physics Messages (2, 14, 43, 124, 125)

### MSG_MOVE_PLAYER (2)

**Direction:** Client → Server, Server → Broadcast  
**Purpose:** Synchronize player movement

**Client → Server:**
```rust
struct MovementUpdate {
    msg_type: u16,  // 2
    direction: u8,  // See direction codes below
    x: Option<u16>, // Only for certain directions
    y: Option<u16>, // Only for certain directions
}

// Direction codes:
// 1  = Start left (ground) + x,y
// 2  = Start right (ground) + x,y
// 3  = Jump + x only
// 4  = Duck (no coords)
// 5  = Stop left (ground) + x,y
// 6  = Stop right (ground) + x,y
// 7  = Release jump (no coords)
// 8  = Release duck (no coords)
// 9  = Landing + x,y
// 10 = Start left (air, no coords)
// 11 = Start right (air, no coords)
// 12 = Stop left (air, no coords)
// 13 = Stop right (air, no coords)
```

**Server → Broadcast (to room):**
```rust
struct PlayerMoved {
    msg_type: u16,  // 2
    player_id: u16,
    direction: u8,
    x: Option<u16>,
    y: Option<u16>,
}
```

**Validation:**
- Verify position within room bounds
- Check physics constraints (can't jump while mid-air, etc.)
- Detect teleporting (sudden position changes)
- Rate limit: max 60 updates/second per player

---

### MSG_PLAYER_STOP (43)

**Direction:** Server → Client  
**Purpose:** Force player to stop moving

**Server → Client:**
```rust
struct ForceStop {
    msg_type: u16,  // 43
}
```

**When Sent:**
- Server detects invalid movement
- Player enters cutscene/dialogue
- Anti-cheat triggers

---

## Communication Messages (17, 133)

### MSG_CHAT (17)

**Direction:** Client → Server, Server → Broadcast  
**Purpose:** Text chat

**Client → Server:**
```rust
struct ChatMessage {
    msg_type: u16,  // 17
    message: String, // Max 100 characters
}
```

**Server → Broadcast (to room):**
```rust
struct ChatBroadcast {
    msg_type: u16,  // 17
    player_id: u16,
    message: String,
}
```

**Validation:**
- Max 100 characters
- Filter profanity (configurable)
- Rate limit: 1 message per 2 seconds
- Spam detection: same message 3x = warning

---

### MSG_PLAYER_TYPING (133)

**Direction:** Client → Server, Server → Broadcast  
**Purpose:** Show typing indicator

**Client → Server:**
```rust
struct TypingIndicator {
    msg_type: u16,  // 133
}
```

**Server → Broadcast (to room):**
```rust
struct PlayerTyping {
    msg_type: u16,  // 133
    player_id: u16,
}
```

---

## Items & Inventory Messages (31-41)

### MSG_USE_ITEM (31)

**Direction:** Client → Server  
**Purpose:** Use item from inventory

**Client → Server:**
```rust
struct UseItem {
    msg_type: u16,  // 31
    slot: u8,       // 1-9
}
```

**Server Response:** Varies by item type
- Consumable: Update points, remove item, send effect messages
- Warp-Wing: Send MSG_WARP
- Seed: Check if at planting spot
- etc.

---

### MSG_DISCARD_ITEM (39)

**Direction:** Client → Server, Server → Broadcast  
**Purpose:** Drop item on ground

**Client → Server:**
```rust
struct DiscardItem {
    msg_type: u16,  // 39
    slot: u8,       // 1-9
}
```

**Server → Broadcast (to room):**
```rust
struct ItemDiscarded {
    msg_type: u16,  // 39
    x: u16,
    y: u16,
    item_id: u16,
    instance_id: u16, // Unique ID for this dropped item
}
```

---

### MSG_DISCARDED_ITEM_TAKE (40)

**Direction:** Client → Server  
**Purpose:** Pick up dropped item

**Client → Server:**
```rust
struct TakeDiscardedItem {
    msg_type: u16,  // 40
    instance_id: u16,
}
```

**Server Response:**
- If successful: MSG_GET_ITEM
- If failed (item gone, inv full): No response

---

### MSG_GET_ITEM (41)

**Direction:** Server → Client  
**Purpose:** Add item to inventory

**Server → Client:**
```rust
struct GetItem {
    msg_type: u16,  // 41
    slot: u8,       // 1-9
    item_id: u16,
}
```

---

## Shop & Economy Messages (27-30, 53-55)

### MSG_ROOM_SHOP_INFO (27)

**Direction:** Server → Client  
**Purpose:** Send shop inventory when player enters shop

**Server → Client:**
```rust
struct ShopInfo {
    msg_type: u16,  // 27
    shop_id: u16,
    category: u8,   // 1=outfits, 2=items, 3=accessories, 4=tools
    items: Vec<ShopItem>,
}

struct ShopItem {
    slot: u8,       // Display position
    item_id: u16,
    price: u16,
    stock: u16,     // 0 = unlimited
}
```

---

### MSG_SHOP_BUY (28)

**Direction:** Client → Server, Server → Client  
**Purpose:** Purchase item from shop

**Client → Server:**
```rust
struct BuyRequest {
    msg_type: u16,  // 28
    category: u8,
    item_id: u16,
}
```

**Server → Client (Success):**
```rust
struct BuySuccess {
    msg_type: u16,  // 28
    category: u8,   // 1=outfit, 2=item, 3=acs, 4=tool
    slot: u8,       // Inventory slot it went to
    item_id: u16,
    new_points: u16, // Remaining points
}
```

**Server → Client (Failure):**
- MSG_SHOP_BUY_FAIL (29)

---

### MSG_SHOP_BUY_FAIL (29)

**Direction:** Server → Client  
**Purpose:** Purchase failed

**Server → Client:**
```rust
struct BuyFailed {
    msg_type: u16,  // 29
    reason: u8,     // 1=insufficient funds, 2=inv full, 3=out of stock
}
```

---

### MSG_SELL_REQ_PRICES (53)

**Direction:** Client → Server, Server → Client  
**Purpose:** Request/respond with sell prices for inventory

**Client → Server:**
```rust
struct SellPricesRequest {
    msg_type: u16,  // 53
    category: u8,   // 1=outfits, 2=items, 3=acs, 4=tools
}
```

**Server → Client:**
```rust
struct SellPricesResponse {
    msg_type: u16,  // 53
    // For each non-empty slot in the category, send price
    // Prices are: round(buy_price / 3)
    prices: Vec<u16>,  // Only for slots with items
}
```

**Price Calculation:**
```rust
sell_price = (buy_price as f32 / 3.0).round() as u16
```

---

## Message Summary Table

| ID | Name | C→S | S→C | Category |
|----|------|-----|-----|----------|
| 1 | MSG_NEW_PLAYER | ✓ | ✓ | Player |
| 2 | MSG_MOVE_PLAYER | ✓ | ✓ | Movement |
| 3 | (reserved) | - | - | - |
| 4 | (reserved) | - | - | - |
| 5 | MSG_CHANGE_ROOM | ✓ | - | Player |
| 6 | MSG_LOGOUT | ✓ | ✓ | Player |
| 7 | MSG_REGISTER | ✓ | ✓ | Auth |
| 8 | (reserved) | - | - | - |
| 9 | MSG_PING | ✓ | ✓ | Utility |
| 10 | MSG_LOGIN | ✓ | ✓ | Auth |
| 11 | MSG_LOGOUT | ✓ | - | Player |
| 17 | MSG_CHAT | ✓ | ✓ | Chat |
| 31 | MSG_USE_ITEM | ✓ | - | Items |
| 39 | MSG_DISCARD_ITEM | ✓ | ✓ | Items |
| 40 | MSG_DISCARDED_ITEM_TAKE | ✓ | - | Items |
| 41 | MSG_GET_ITEM | - | ✓ | Items |
| 45 | MSG_BANK_PROCESS | ✓ | ✓ | Banking |
| 47 | MSG_MAILBOX | ✓ | ✓ | Mail |
| ... | (118 more messages) | | | |

*Full catalog continues in implementation-specific message handler files*

---

## Implementation Priority

**Phase 1 (Core):**
- MSG_LOGIN (10)
- MSG_REGISTER (7)
- MSG_NEW_PLAYER (1)
- MSG_LOGOUT (6)
- MSG_PING (9)

**Phase 2 (Gameplay):**
- MSG_MOVE_PLAYER (2)
- MSG_CHAT (17)
- MSG_WARP (14)
- MSG_USE_ITEM (31)
- MSG_GET_ITEM (41)

**Phase 3 (Features):**
- Shop messages (27-30)
- Quest messages (83-92)
- Clan messages (126-132)

**Phase 4 (Advanced):**
- BBS messages (134-141)
- Planting messages (63-72)
- Special features (cannons, racing, etc.)

---

**Next:** Refer to individual protocol files for detailed message specifications.
