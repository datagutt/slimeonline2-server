# Implementation Plan: Server Data Updates

Based on analysis of the original server source code, this document outlines changes needed to update the Rust server implementation with accurate game data.

---

## Current Implementation Status

### Messages Implemented: 44 of 141

| Category | Implemented | Notes |
|----------|-------------|-------|
| Authentication | MSG_LOGIN, MSG_REGISTER, MSG_LOGOUT | Complete |
| Movement | MSG_MOVE_PLAYER, MSG_PLAYER_STOP, MSG_NEW_PLAYER | Complete |
| Chat | MSG_CHAT | Complete with rate limiting |
| Appearance | MSG_CHANGE_OUTFIT, MSG_CHANGE_ACS1/2, MSG_EMOTE, MSG_ACTION | Complete |
| Warp | MSG_WARP | Complete |
| Items | MSG_USE_ITEM, MSG_DISCARD, MSG_DISCARDED_TAKE | Partial (seeds/fairies incomplete) |
| Collectibles | MSG_COLLECTIBLE_INFO/TAKE/TAKEN | Hardcoded spawns, needs config |
| Shop | MSG_SHOP_INFO, MSG_SHOP_BUY, MSG_SELL, MSG_SELL_PRICES | Complete |
| Bank | MSG_BANK_PROCESS, MSG_REQUEST_STATUS | Complete |
| Mail | MSG_MAILBOX (all ops), MSG_MAIL_SEND, MSG_RECEIVER_CHECK | Complete |
| Tools | MSG_TOOL_EQUIP/UNEQUIP | Complete |
| BBS | All 8 BBS messages | Complete |
| Utility | MSG_PING, MSG_PING_REQ, MSG_SAVE, MSG_POINT | Complete |

### Major Systems NOT Implemented (97 messages remaining)

| System | Messages | Priority |
|--------|----------|----------|
| **Clan System** | 6 messages (126-131) | High |
| **Quest System** | 10 messages (83-92) | High |
| **Planting System** | 9 messages (63-70, 94) | Medium |
| **Storage Extension** | 3 messages (56-58) | Medium |
| **Building System** | 4 messages (103-106) | Low |
| **Cannon System** | 4 messages (98-101) | Low |
| **Racing System** | 6 messages (120-125) | Low |
| **Upgrader System** | 5 messages (108-112) | Low |

---

## Protocol Details from Original Server

### Authentication Corrections

**MSG_LOGIN fields (in order):**
```
Client sends:
  - version: string (must match "0.106")
  - username: string (converted to lowercase)
  - password: string (converted to lowercase)
  - mac_address: string

Special case - Mod Login (version == "ModAccess"):
  - mod_name: string
  - mod_password: string
```

**Login result codes:**
| Code | Meaning |
|------|---------|
| 1 | Success |
| 2 | Account does not exist |
| 3 | Wrong password / Already logged in (mod) |
| 4 | Already logged in (player) |
| 5 | Version mismatch |
| 6 | Account banned |
| 7 | IP banned |
| 8 | MAC banned |

**MSG_REGISTER validation:**
- Username max 10 chars (not 20 as in docs - VERIFY THIS)
- Password max 10 chars (not 50 as in docs - VERIFY THIS)
- Both stored lowercase
- Silent rejection if length exceeded

### Movement Protocol Details

**MSG_MOVE_PLAYER direction codes:**
| Code | Action | Extra data |
|------|--------|-----------|
| 1 | Move left | x: u16, y: u16 |
| 2 | Move right | x: u16, y: u16 |
| 3 | Jump | x: i16 (SIGNED!) |
| 4 | Duck | none |
| 5 | Release left | x: u16, y: u16 |
| 6 | Release right | x: u16, y: u16 |
| 7 | Release up | none |
| 8 | Release down | none |
| 9 | Land | x: u16, y: u16 |
| 10-13 | Air movement | none |

**Important:** Direction 3 (jump) uses a **signed i16** for x, not unsigned!

### Economy Corrections

**Sell formula:** `sell_price = buy_price / 3` (integer division, rounded down)

**Bank overflow handling:** If points would exceed MAX_POINTS after operation, excess automatically goes to bank.

**MSG_SELL format:**
```
Client sends:
  - category: u8 (1=Outfits, 2=Items, 3=Acs, 4=Tools)
  - count: u8 (number of items)
  - slots[]: u8 (repeated 'count' times)

Server responds:
  - msg_type: u16
  - total_received: u32
```

### Clan Protocol Details

**MSG_CLAN_CREATE requirements:**
- Player not already in clan
- Name 3-15 characters
- Unique name (case-insensitive)
- Has item 51 (Proof of Nature)
- Has item 52 (Proof of Earth)
- Has 10,000 SP

**MSG_CLAN_INFO types:**
| Type | Action |
|------|--------|
| 1 | Get clan name/color by clan_id |
| 2 | Get member list |
| 3 | Get status (sub_type: 1=points only, 2=full) |
| 4 | Get info text (leader only) |
| 5 | Get clan news |

**MSG_CLAN_ADMIN actions:**
| Action | Data |
|--------|------|
| 1 | Kick: member_slot: u8 |
| 2 | Invite: target_pid: u16 (15s cooldown per target) |
| 3 | Colors: inner_rgb + outer_rgb (6 bytes) |
| 4 | Update info: show_leader: u8 + info_text: string |
| 5 | Update news: news_text: string |

### Mail Protocol Details

**MSG_MAIL_SEND format:**
```
Client sends:
  - paper_id: u8
  - font_color: u8
  - receiver_name: string (lowercase)
  - present_category: u8 (0=none, 1-4=category)
  - present_id: u16 (slot index, not item ID!)
  - attached_points: u16 (0-60000 max)
  - mail_text: string
```

**Mailbox limits:** 50 mails maximum

### Quest Protocol Details

**MSG_QUEST_REWARD format:**
```
Client sends:
  - quest_id: u8
  - quest_step: u8

Validation:
  - Player must have active quest
  - quest_id must match current
  - quest_step must match current
  - Quest-specific item requirements checked
```

### Collectible Evolution

Collectibles can transform over time:
```
Item 20 (Red Mushroom) -> Item 58 (Squishy Mushroom) after 60±20 min
Item 58 (Squishy Mushroom) -> Item 59 (Stinky Mushroom) after 10±20 min
```

### Special Emote Handling

**Dice emote (id 13):** Server generates random result 1-6, not client.

---

## Hardcoded Values to Replace with Config

### Critical Fixes

| Location | Current Value | Config Source |
|----------|---------------|---------------|
| `constants.rs:252-259` | spawn x=160, y=120, room=37 | `game.toml` spawn_x=385, y=71, room=32 |
| `handlers/bbs.rs:15-21` | 5 categories | `game.toml` 6 categories |
| `handlers/bbs.rs:24` | 60s cooldown | Should be configurable |
| `handlers/collectibles.rs` | Test spawns only | `collectibles.toml` (47 rooms) |
| `handlers/connection.rs:350` | All mail paper free | `game.toml` unlocked_mail_paper |

---

## Design Principle: Config Files as Single Source of Truth

**Static game data** (prices, spawn points, growth rates, etc.) should be defined in **config files only** - NOT in the database. This avoids duplication and makes the server easy to configure.

**Database is for:**
- Player accounts and characters
- Runtime state (current collectible availability, plant growth progress)
- Session data
- Logs/bans

**Config files are for:**
- Item/outfit/accessory/tool prices
- Shop inventories per room
- Collectible spawn definitions
- Plant growth configuration
- Clan creation requirements
- All other static game rules

---

## Config File Structure

```
config/
├── server.toml              # Server settings (port, db path, etc.)
├── game.toml                # General game rules (limits, defaults)
├── prices.toml              # All item/outfit/accessory/tool prices
├── shops.toml               # Shop inventories per room
├── collectibles.toml        # Collectible spawn points per room
├── plants.toml              # Plant growth configuration
└── clans.toml               # Clan system configuration
```

---

## Config File Definitions

### server.toml
```toml
[server]
port = 5555
database_path = "slime_online2.db"
max_connections = 500

[logging]
level = "info"
```

### game.toml
```toml
[limits]
max_username_length = 20
min_username_length = 3
max_password_length = 50
min_password_length = 6
max_chat_length = 100
max_points = 10000000
max_bank_balance = 100000000

[defaults]
spawn_x = 385
spawn_y = 71
spawn_room = 32
outfit = 1
signature = 1
signature_bg = 1
emotes = [5, 4, 6, 8, 7]
starting_items = [1, 1, 1, 24]  # Fly Wing x3, Blue Seed
welcome_mail_points = 50
```

### prices.toml
```toml
# Sell price = buy_price / 3 (rounded)
# Items with price 65000 are special/non-purchasable

[items]
1 = { name = "Fly Wing", price = 10 }
2 = { name = "Smoke Bomb", price = 30 }
3 = { name = "Apple Bomb", price = 15 }
4 = { name = "Bubble Wand", price = 15 }
5 = { name = "Points Bag [50]", price = 50 }
6 = { name = "Points Bag [200]", price = 200 }
7 = { name = "Points Bag [500]", price = 500 }
8 = { name = "Chicken Mine", price = 10 }
9 = { name = "Simple Seed", price = 150 }
10 = { name = "Fairy", price = 250 }
11 = { name = "Blue Pinwheel", price = 100 }
12 = { name = "Red Pinwheel", price = 250 }
13 = { name = "Glow Pinwheel", price = 500 }
14 = { name = "Rockman Sound", price = 1500 }
15 = { name = "Kirby Sound", price = 1500 }
16 = { name = "Link Sound", price = 1500 }
17 = { name = "Pipe Sound", price = 1500 }
18 = { name = "DK Sound", price = 1500 }
19 = { name = "Metroid Sound", price = 1500 }
20 = { name = "Red Mushroom", price = 50 }
21 = { name = "Tailphire", price = 300 }
22 = { name = "Magmanis", price = 30 }
23 = { name = "Bright Drink", price = 150 }
24 = { name = "Blue Seed", price = 150 }
25 = { name = "Juicy Bango", price = 450 }
26 = { name = "Weak Cannon Kit", price = 900 }
27 = { name = "Red Gum", price = 15 }
28 = { name = "Orange Gum", price = 15 }
29 = { name = "Green Gum", price = 15 }
30 = { name = "Blue Gum", price = 15 }
31 = { name = "Pink Gum", price = 15 }
32 = { name = "White Gum", price = 15 }
33 = { name = "Lucky Coin", price = 3 }
34 = { name = "Bunny Soda", price = 30 }
35 = { name = "Slime Soda", price = 30 }
36 = { name = "Penguin Soda", price = 30 }
37 = { name = "Speed Soda", price = 3 }
38 = { name = "Jump Soda", price = 3 }
39 = { name = "Sleenmium", price = 60 }
40 = { name = "Sledmium", price = 60 }
41 = { name = "Sluemium", price = 60 }
42 = { name = "Slinkmium", price = 60 }
43 = { name = "Slelloymium", price = 60 }
44 = { name = "Slaymium", price = 60 }
45 = { name = "Slackmium", price = 60 }
46 = { name = "Screw", price = 15 }
47 = { name = "Rusty Screw", price = 6 }
48 = { name = "Bug Leg", price = 3 }
49 = { name = "Weird Coin", price = 75 }
50 = { name = "Firestone", price = 30 }
51 = { name = "Proof of Nature", price = 3 }
52 = { name = "Proof of Earth", price = 3 }
53 = { name = "Proof of Water", price = 3 }
54 = { name = "Proof of Fire", price = 3 }
55 = { name = "Proof of Stone", price = 3 }
56 = { name = "Proof of Wind", price = 3 }
57 = { name = "Blazing Bubble", price = 150 }
58 = { name = "Squishy Mushroom", price = 150 }
59 = { name = "Stinky Mushroom", price = 3 }
60 = { name = "Bell Twig", price = 30 }
61 = { name = "Irrlicht", price = 300 }

[outfits]
1 = 50
2 = 200
3 = 1000
4 = 1000
5 = 300
6 = 2000
7 = 1500
8 = 3000
9 = 1000
10 = 800
# ... continue for all 100 outfits
92 = 1337  # Easter egg price

[accessories]
1 = 100
2 = 2000
3 = 500
# ... continue for all 101 accessories

[tools]
1 = { name = "Rusty Pickaxe", price = 500 }
2 = { name = "Pickaxe", price = 1000 }

[mail_paper]
0 = 25
1 = 40
2 = 100
3 = 50

# Items that can be discarded (dropped on ground)
# All items 1-61 are discardable by default
discardable = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61]
```

### shops.toml
```toml
# Shop inventories per room
# Each shop slot: category (1=outfit, 2=item, 3=acs, 4=tool), item_id, max_stock (0=unlimited)

[room.44]
slots = [
    { cat = 1, item = 15, stock = 10 },
    { cat = 1, item = 38, stock = 2 },
    { cat = 1, item = 44, stock = 1 },
    { cat = 1, item = 57, stock = 2 },
]

[room.45]
slots = [
    { cat = 2, item = 1, stock = 0 },   # Fly Wing, unlimited
    { cat = 2, item = 9, stock = 5 },   # Simple Seed
]

# ... more rooms
```

### collectibles.toml
```toml
# Collectible spawn points per room
# respawn = base minutes, variance = random 0-N additional minutes

[room.100]
spawns = [
    { id = 1, item = 22, x = 196, y = 56, respawn = 20, variance = 20 },
    { id = 2, item = 22, x = 112, y = 376, respawn = 30, variance = 20 },
    { id = 3, item = 57, x = 176, y = 64, respawn = 40, variance = 30 },
]

[room.101]
spawns = [
    { id = 1, item = 22, x = 488, y = 216, respawn = 30, variance = 15 },
    { id = 2, item = 22, x = 520, y = 216, respawn = 30, variance = 10 },
    { id = 3, item = 57, x = 416, y = 264, respawn = 50, variance = 50 },
    { id = 4, item = 57, x = 192, y = 200, respawn = 60, variance = 20 },
]

[room.102]
spawns = [
    { id = 1, item = 22, x = 136, y = 456, respawn = 30, variance = 30 },
    { id = 2, item = 22, x = 320, y = 344, respawn = 30, variance = 10 },
    { id = 3, item = 22, x = 160, y = 56, respawn = 15, variance = 35 },
    { id = 4, item = 22, x = 464, y = 88, respawn = 30, variance = 20 },
    { id = 5, item = 57, x = 608, y = 320, respawn = 30, variance = 70 },
    { id = 6, item = 22, x = 912, y = 104, respawn = 20, variance = 40 },
]

[room.109]
spawns = [
    { id = 1, item = 57, x = 192, y = 568, respawn = 60, variance = 60 },
    { id = 2, item = 57, x = 296, y = 288, respawn = 60, variance = 30 },
    { id = 3, item = 57, x = 864, y = 552, respawn = 40, variance = 60 },
    { id = 4, item = 22, x = 376, y = 280, respawn = 30, variance = 10 },
    { id = 5, item = 57, x = 896, y = 328, respawn = 40, variance = 80 },
    { id = 6, item = 57, x = 856, y = 136, respawn = 80, variance = 80 },
]

[room.115]
spawns = [
    { id = 1, item = 60, x = 704, y = 392, respawn = 30, variance = 60 },
]

[room.116]
spawns = [
    { id = 1, item = 60, x = 496, y = 184, respawn = 20, variance = 40 },
]

[room.117]
spawns = [
    { id = 1, item = 60, x = 600, y = 616, respawn = 80, variance = 80 },
]

[room.119]
spawns = [
    { id = 1, item = 60, x = 336, y = 104, respawn = 80, variance = 100 },
]

[room.120]
spawns = [
    { id = 1, item = 60, x = 248, y = 472, respawn = 30, variance = 80 },
]

# Evolving collectibles - item transforms after time
[evolving]
20 = { to = 58, minutes = 60, variance = 20 }  # Red Mushroom -> Squishy Mushroom
58 = { to = 59, minutes = 10, variance = 20 }  # Squishy Mushroom -> Stinky Mushroom
```

### plants.toml
```toml
# Plant growth configuration
# stages = [stage1, stage2, stage3, stage4, fruit_duration, death_duration] in minutes
# fruits = possible fruit item IDs (equal chance each)
# chance = base % chance for fruit (0-100)

[seeds.9]
name = "Simple Seed"
stages = [240, 240, 360, 360, 720, 60]
fruits = [9, 9, 9, 9, 9]
chance = 50

[seeds.24]
name = "Blue Seed"
stages = [240, 360, 360, 480, 720, 60]
fruits = [25, 25, 24, 25, 25]
chance = 35

# Fairy bonus: +10% fruit chance per fairy, max 5 fairies
[bonuses]
fairy_chance_bonus = 10
max_fairies = 5
```

### clans.toml
```toml
[creation]
cost = 10000
required_items = [51, 52]  # Proof of Nature, Proof of Earth

[limits]
min_name_length = 3
max_name_length = 15
initial_member_slots = 3
max_member_slots = 10

[bbs_categories]
0 = "Hints"
1 = "Events"
2 = "Trade"
3 = "Adventures"
4 = "Jokes"
5 = "Other"
```

---

## Code Changes Required

### 1. Create Config Module (`src/config/mod.rs`)

```rust
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct GameConfig {
    pub server: ServerConfig,
    pub game: GameRules,
    pub prices: PriceConfig,
    pub shops: HashMap<u16, ShopConfig>,
    pub collectibles: CollectibleConfig,
    pub plants: PlantConfig,
    pub clans: ClanConfig,
}

// Load all config files on startup
pub fn load_config(config_dir: &str) -> Result<GameConfig, ConfigError> {
    // ...
}
```

### 2. Update Server Struct

```rust
pub struct Server {
    pub config: Arc<GameConfig>,  // Add config
    pub db: DbPool,
    pub sessions: DashMap<...>,
    // ...
}
```

### 3. Update Handlers to Use Config

**Shop buy:**
```rust
let price = server.config.prices.items.get(&item_id)
    .map(|p| p.price)
    .ok_or(ShopError::InvalidItem)?;
```

**Collectibles:**
```rust
let spawns = server.config.collectibles.rooms.get(&room_id)
    .map(|r| &r.spawns)
    .unwrap_or(&vec![]);
```

### 4. Database Only Stores Runtime State

```sql
-- Collectible runtime state (which are currently available)
CREATE TABLE collectible_state (
    room_id INTEGER NOT NULL,
    spawn_id INTEGER NOT NULL,
    available INTEGER DEFAULT 1,
    respawn_at TEXT,  -- datetime when it respawns
    PRIMARY KEY (room_id, spawn_id)
);

-- Plant runtime state
CREATE TABLE plant_state (
    room_id INTEGER NOT NULL,
    spot_id INTEGER NOT NULL,
    owner_id INTEGER,
    seed_id INTEGER,
    stage INTEGER DEFAULT 0,
    fairy_count INTEGER DEFAULT 0,
    next_stage_at TEXT,
    PRIMARY KEY (room_id, spot_id)
);

-- Shop stock runtime state (for limited items)
CREATE TABLE shop_stock (
    room_id INTEGER NOT NULL,
    slot_id INTEGER NOT NULL,
    current_stock INTEGER,
    last_restock TEXT,
    PRIMARY KEY (room_id, slot_id)
);
```

---

## Priority Order

### Phase 1: Config System
- [ ] Create config file structure
- [ ] Implement config loader with serde
- [ ] Add config to Server struct
- [ ] Validate config on startup

### Phase 2: Prices
- [ ] Create `prices.toml` with all data
- [ ] Update shop buy handler to use config
- [ ] Update sell handler to calculate from config
- [ ] Remove hardcoded prices

### Phase 3: Collectibles
- [ ] Create `collectibles.toml` with all spawn data
- [ ] Update collectibles handler to use config
- [ ] Add runtime state table for respawn tracking
- [ ] Remove hardcoded `get_room_collectibles()`

### Phase 4: Shops
- [ ] Create `shops.toml` with room shop data
- [ ] Add shop stock runtime table
- [ ] Implement daily stock reset

### Phase 5: Plants & Clans
- [ ] Create `plants.toml`
- [ ] Create `clans.toml`
- [ ] Update respective handlers

---

## Files to Create

```
config/
├── server.toml
├── game.toml
├── prices.toml
├── shops.toml
├── collectibles.toml
├── plants.toml
└── clans.toml

src/config/
├── mod.rs
├── loader.rs
├── types.rs
└── validation.rs
```

## Files to Modify

- `src/main.rs` - Load config on startup
- `src/lib.rs` - Add config module
- `src/handlers/shop/buy.rs` - Use config prices
- `src/handlers/shop/sell.rs` - Use config prices
- `src/handlers/collectibles.rs` - Use config spawns
- `src/game/mod.rs` - Initialize from config

---

## Implementation Roadmap

### Phase 1: Config System (Current Priority)
- [ ] Create `src/config/` module with serde types
- [ ] Load all TOML config files on startup
- [ ] Replace hardcoded spawn values with config
- [ ] Replace hardcoded BBS categories with config
- [ ] Add config validation

### Phase 2: Fix Existing Handlers
- [ ] Update collectibles to use `collectibles.toml`
- [ ] Fix movement direction 3 to use signed i16
- [ ] Add bank overflow handling
- [ ] Fix mail paper availability from config

### Phase 3: Implement Clan System (6 messages)
- [ ] MSG_CLAN_CREATE (126) - Create clan with requirements
- [ ] MSG_CLAN_DISSOLVE (127) - Leader dissolves clan
- [ ] MSG_CLAN_INVITE (128) - Accept/decline with cooldown
- [ ] MSG_CLAN_LEAVE (129) - Leave clan
- [ ] MSG_CLAN_INFO (130) - 5 sub-types for info requests
- [ ] MSG_CLAN_ADMIN (131) - 5 admin actions
- [ ] Database: Create clans table, member relationships
- [ ] File storage: Clan data files (or migrate to DB-only)

### Phase 4: Implement Quest System (10 messages)
- [ ] MSG_QUEST_BEGIN (83)
- [ ] MSG_QUEST_CLEAR (84)
- [ ] MSG_QUEST_STEP_INC (85)
- [ ] MSG_QUEST_CANCEL (86)
- [ ] MSG_QUEST_NPC_REQ (87)
- [ ] MSG_QUEST_VAR_CHECK (88)
- [ ] MSG_QUEST_VAR_INC (89)
- [ ] MSG_QUEST_VAR_SET (90)
- [ ] MSG_QUEST_STATUS_REQ (91)
- [ ] MSG_QUEST_REWARD (92)
- [ ] Create quest definitions config
- [ ] Database: Quest progress tracking

### Phase 5: Implement Planting System (9 messages)
- [ ] MSG_PLANT_SET - Plant seed
- [ ] MSG_PLANT_TAKE_FRUIT - Harvest
- [ ] MSG_PLANT_ADD_FAIRY - Add fairy (max 5)
- [ ] MSG_PLANT_ADD_PINWHEEL - Add pinwheel
- [ ] Plant growth timer system
- [ ] Fruit chance calculation with fairy bonus
- [ ] Database: Plant state table

### Phase 6: Storage System (3 messages)
- [ ] MSG_STORAGE_REQ (56) - Open storage
- [ ] MSG_STORAGE_PAGES (57) - Page navigation
- [ ] MSG_STORAGE_MOVE (58) - Move items between storage/inventory
- [ ] Database: Storage tables per category

### Phase 7: Secondary Systems (Lower Priority)
- [ ] Building System (4 messages)
- [ ] Cannon System (4 messages)
- [ ] Racing System (6 messages)
- [ ] Upgrader System (5 messages)
- [ ] One-Time Items (3 messages)
- [ ] Music Changer (2 messages)

---

## Validation Patterns to Implement

From original server analysis:

1. **Slot bounds:** All inventory operations must check slot is 1-9
2. **Empty slot:** Verify slot has content before operations
3. **Room context:** Verify room has required feature (shop, plants, etc.)
4. **Ownership:** Plants/clan operations verify player ownership
5. **Currency:** Always verify sufficient funds BEFORE deducting
6. **Free slot:** Item acquisition must verify free slot exists
7. **Hack alerts:** Log suspicious activity patterns

---

## Testing Checklist

### With Real v0.106 Client
- [ ] Login/logout cycle
- [ ] Character creation with correct spawn point
- [ ] Movement synchronization
- [ ] Chat in rooms
- [ ] Shop buy/sell with correct prices
- [ ] Collectible spawns in correct locations
- [ ] Mail send/receive with attachments
- [ ] BBS post/read
- [ ] Clan creation (when implemented)
- [ ] Quest progression (when implemented)
