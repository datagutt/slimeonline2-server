# Slime Online 2 Server - Implementation Progress

## Overview

This document tracks the implementation progress of the Slime Online 2 private server.

**Last Updated:** 2024-12-20

## Phase 1: Foundation (Week 1-2)

### Completed

| Task | Status | File(s) |
|------|--------|---------|
| Project setup with Cargo.toml | Done | `Cargo.toml` |
| Directory structure | Done | `src/` |
| RC4 encryption/decryption | Done | `src/crypto.rs` |
| MessageReader (binary parsing) | Done | `src/protocol/reader.rs` |
| MessageWriter (binary serialization) | Done | `src/protocol/writer.rs` |
| Message structures | Done | `src/protocol/messages.rs` |
| Game constants (all 141 message types) | Done | `src/constants.rs` |
| TCP server with tokio | Done | `src/main.rs` |
| Connection handling | Done | `src/handlers/connection.rs` |
| Session management | Done | `src/game/mod.rs` |
| Database schema (SQLite) | Done | `migrations/*.sql` |
| Account database operations | Done | `src/db/accounts.rs` |
| Character database operations | Done | `src/db/characters.rs` |
| MSG_LOGIN handler | Done | `src/handlers/auth.rs` |
| MSG_REGISTER handler | Done | `src/handlers/auth.rs` |
| MSG_PING handler | Done | `src/handlers/connection.rs` |
| MSG_PING_REQ handler | Done | `src/handlers/connection.rs` |
| MSG_LOGOUT handler | Done | `src/handlers/connection.rs` |
| bcrypt password hashing | Done | `src/handlers/auth.rs` |
| Per-IP connection limiting | Done | `src/main.rs` |
| Connection timeout handling | Done | `src/handlers/connection.rs` |
| Periodic save task | Done | `src/main.rs` |
| Stale session cleanup | Done | `src/main.rs` |

### Phase 1 Testing Checklist

- [x] Client can connect to server on port 5555
- [x] Client can register new account
- [x] Client can login with registered account
- [x] Server rejects wrong password
- [x] Server responds to PING messages
- [x] Server responds to PING_REQ for latency measurement (F11)
- [x] Connection times out after inactivity

## Phase 2: Core Gameplay (Week 3-5)

### Completed

| Task | Status | File(s) |
|------|--------|---------|
| MSG_MOVE_PLAYER handler | Done | `src/handlers/movement.rs` |
| MSG_CHAT handler | Done | `src/handlers/chat.rs` |
| MSG_PLAYER_TYPING handler | Done | `src/handlers/chat.rs` |
| MSG_EMOTE handler | Done | `src/handlers/chat.rs` |
| MSG_ACTION handler | Done | `src/handlers/chat.rs` |
| MSG_CHANGE_OUT handler | Done | `src/handlers/appearance.rs` |
| MSG_CHANGE_ACS1 handler | Done | `src/handlers/appearance.rs` |
| MSG_CHANGE_ACS2 handler | Done | `src/handlers/appearance.rs` |
| MSG_POINT handler | Done | `src/handlers/gameplay.rs` |
| MSG_WARP handler | Done | `src/handlers/warp.rs` |
| MSG_PLAYER_STOP handler | Done | `src/handlers/connection.rs` |
| Room player tracking | Done | `src/game/mod.rs` |
| Player broadcast to room | Done | Various handlers |
| New player notification | Done | `src/handlers/auth.rs` |
| Player leave notification | Done | `src/handlers/connection.rs` |
| Points persistence | Done | `src/handlers/gameplay.rs`, `src/db/characters.rs` |
| Room/position persistence | Done | `src/handlers/warp.rs`, `src/db/characters.rs` |
| Outfit/accessory persistence | Done | `src/handlers/appearance.rs`, `src/db/characters.rs` |

### Pending

| Task | Status | Notes |
|------|--------|-------|
| Movement validation | Pending | Physics-based validation |
| Chat profanity filter | Pending | Configurable word list |
| Rate limiting per message type | Pending | Prevent spam |

## Phase 3: Items & Economy (Week 6-8)

### Completed

| Task | Status | File(s) |
|------|--------|---------|
| Item database (61 items) | Done | `src/handlers/items/database.rs` |
| MSG_USE_ITEM handler | Done | `src/handlers/items/use_item.rs` |
| MSG_DISCARD_ITEM handler | Done | `src/handlers/items/discard.rs` |
| MSG_DISCARDED_ITEM_TAKE handler | Done | `src/handlers/items/pickup.rs` |
| Dropped item tracking | Done | `src/game/mod.rs` |
| Shop database (dynamic) | Done | `migrations/20240101000006_create_shops.sql` |
| MSG_SHOP_BUY handler | Done | `src/handlers/shop/buy.rs` |
| MSG_ROOM_SHOP_INFO sender | Done | `src/handlers/shop/buy.rs` |
| Shop info on room enter | Done | `src/handlers/warp.rs`, `src/handlers/auth.rs` |
| Visual effects broadcast (smokebomb, bubbles, etc.) | Done | `src/handlers/items/use_item.rs` |
| MSG_REQUEST_STATUS handler | Done | `src/handlers/bank.rs` |
| MSG_BANK_PROCESS handler (deposit/withdraw/transfer) | Done | `src/handlers/bank.rs` |
| Bank transfer with transaction rollback | Done | `src/db/characters.rs` |
| MSG_TOOL_EQUIP / MSG_TOOL_UNEQUIP handlers | Done | `src/handlers/connection.rs` |
| MSG_SELL_REQ_PRICES handler | Done | `src/handlers/shop/sell.rs` |
| MSG_SELL handler (all categories) | Done | `src/handlers/shop/sell.rs` |
| Sell prices for items/outfits/accessories/tools | Done | `src/handlers/items/database.rs`, `src/handlers/shop/sell.rs` |
| Collectible system (materials pickup) | Done | `src/handlers/collectibles.rs`, `src/game/mod.rs` |
| MSG_COLLECTIBLE_INFO sender (32) | Done | `src/handlers/collectibles.rs` |
| MSG_COLLECTIBLE_TAKE_SELF handler (33) | Done | `src/handlers/collectibles.rs` |
| MSG_COLLECTIBLE_TAKEN broadcast (34) | Done | `src/handlers/collectibles.rs` |
| MSG_GET_ITEM sender (41) | Done | `src/handlers/collectibles.rs` |
| Collectible respawn timer | Done | `src/game/mod.rs` |

### Pending

| Task | Status | Notes |
|------|--------|-------|
| Collectible spawn data from world files | Pending | Parse actual room/collectible positions |

## Phase 4: Social Features (Week 9-11)

### Completed

| Task | Status | File(s) |
|------|--------|---------|
| Mail database schema | Done | `migrations/20240101000007_create_mail.sql` |
| Mail database operations | Done | `src/db/mail.rs` |
| MSG_MAILBOX handler | Done | `src/handlers/mail.rs` |
| MSG_MAIL_SEND handler | Done | `src/handlers/mail.rs` |
| MSG_MAIL_RECEIVER_CHECK handler | Done | `src/handlers/mail.rs` |
| Mail item/points attachments | Done | `src/handlers/mail.rs` |

### Pending

| Task | Status | Notes |
|------|--------|-------|
| Clan creation/dissolution | Pending | MSG_CLAN_CREATE, MSG_CLAN_DISSOLVE |
| Clan invites/kicks | Pending | MSG_CLAN_INVITE, MSG_CLAN_LEAVE |
| BBS system | Pending | MSG_BBS_* handlers |

## Phase 5: Game Systems (Week 12-14)

### Pending

| Task | Status | Notes |
|------|--------|-------|
| Quest system | Pending | MSG_QUEST_* handlers |
| Planting system | Pending | MSG_PLANT_* handlers |
| Building system | Pending | MSG_BUILD_* handlers |
| Cannon system | Pending | MSG_CANNON_* handlers |
| Racing system | Pending | MSG_RACE_* handlers |
| Tool equip/unequip | Pending | MSG_TOOL_EQUIP, MSG_TOOL_UNEQUIP |

## Phase 6: Production (Week 15-16)

### Pending

| Task | Status | Notes |
|------|--------|-------|
| Comprehensive input validation | Pending | Security hardening |
| Rate limiting refinement | Pending | Anti-spam |
| Anti-cheat detection | Pending | Teleport detection, etc. |
| Performance optimization | Pending | Profiling and tuning |
| Metrics/monitoring | Pending | Prometheus integration |
| Admin commands | Pending | Kick, ban, announce |

## Tools

### SOR Tool (`sor_tool/`)

A Rust CLI tool for working with the game's encrypted .sor archive files.

| Command | Description |
|---------|-------------|
| `sor_tool list <archive.sor>` | List files in archive |
| `sor_tool extract <archive.sor> <password> <output_dir>` | Extract files |
| `sor_tool create <input_dir> <password> <output.sor>` | Create new archive |
| `sor_tool rekey <input.sor> <old_pass> <new_pass> <output.sor>` | Re-encrypt with new key |

**Known .sor passwords:**
- `acs.sor` (correct): `ewtrhj654736z2g5q6bzhn6u`
- `backgrounds.sor`: `adfadsfbgh4534ewfgr`

## Architecture

```
src/
├── main.rs              # Entry point, TCP listener, background tasks
├── constants.rs         # All game constants and message types
├── crypto.rs            # RC4 encryption implementation
├── protocol/
│   ├── mod.rs           # Protocol module exports
│   ├── types.rs         # MessageType enum (141 types)
│   ├── reader.rs        # Binary message reader
│   ├── writer.rs        # Binary message writer
│   └── messages.rs      # Message structures
├── handlers/
│   ├── mod.rs           # Handler module exports
│   ├── connection.rs    # Connection lifecycle, message routing, ping, player stop
│   ├── auth.rs          # Login/register handlers
│   ├── movement.rs      # Movement message handler
│   ├── chat.rs          # Chat, emote, action, typing handlers
│   ├── appearance.rs    # Outfit and accessory change handlers
│   ├── gameplay.rs      # Points collection and gameplay handlers
│   ├── warp.rs          # Room change/warp handler
│   ├── items/           # Item system handlers
│   │   ├── mod.rs
│   │   ├── database.rs  # 61 items from db_items.gml
│   │   ├── use_item.rs  # MSG_USE_ITEM + visual effects
│   │   ├── discard.rs   # MSG_DISCARD_ITEM
│   │   └── pickup.rs    # MSG_DISCARDED_ITEM_TAKE
│   ├── shop/            # Shop system handlers
│   │   ├── mod.rs
│   │   ├── buy.rs       # MSG_SHOP_BUY, MSG_ROOM_SHOP_INFO
│   │   └── sell.rs      # MSG_SELL_REQ_PRICES, MSG_SELL
│   ├── bank.rs          # MSG_REQUEST_STATUS, MSG_BANK_PROCESS
│   ├── mail.rs          # MSG_MAILBOX, MSG_MAIL_SEND, MSG_MAIL_RECEIVER_CHECK
│   └── collectibles.rs  # MSG_COLLECTIBLE_INFO, MSG_COLLECTIBLE_TAKE_SELF, MSG_GET_ITEM
├── game/
│   └── mod.rs           # Game state, rooms, sessions, dropped items, collectibles
└── db/
    ├── mod.rs           # Database pool and migrations
    ├── accounts.rs      # Account queries
    ├── characters.rs    # Character queries (position, points, appearance, bank)
    └── mail.rs          # Mail queries (send, get mailbox, claim)

sor_tool/                # SOR archive tool
├── src/main.rs
├── Cargo.toml
└── Cargo.lock

migrations/
├── 20240101000001_create_accounts.sql
├── 20240101000002_create_characters.sql
├── 20240101000003_create_inventories.sql
├── 20240101000004_create_clans.sql
├── 20240101000005_create_bans.sql
├── 20240101000006_create_shops.sql
└── 20240101000007_create_mail.sql
```

## Database Schema

- **accounts**: User authentication (username, password_hash, mac_address, ban status)
- **characters**: Player data (position, appearance, points, bank_balance, quest state)
- **inventories**: Equipment and items (emotes, outfits, accessories, items, tools)
- **clans**: Clan information (name, leader, colors, level)
- **bans**: IP/MAC/account bans
- **shop_items**: Dynamic shop inventory (room_id, slot_id, category, item_id, price, stock)
- **mail**: Player mail (sender, recipient, message, item_id, points, read status)

## Running the Server

```bash
# Build
cargo build

# Run (creates SQLite database automatically)
cargo run

# Server listens on 0.0.0.0:5555
```

## Configuration

Currently uses default configuration in `src/main.rs`:
- Host: 0.0.0.0
- Port: 5555
- Database: sqlite:slime_online2.db
- MOTD: "Welcome to Slime Online 2 Private Server!"

## Message Handlers Implemented

| Message ID | Name | Direction | Status |
|------------|------|-----------|--------|
| 1 | MSG_NEW_PLAYER | S→C, C→S | Done |
| 2 | MSG_MOVE_PLAYER | C→S, S→C | Done |
| 6 | MSG_LOGOUT | C→S, S→C | Done |
| 7 | MSG_REGISTER | C→S, S→C | Done |
| 9 | MSG_PING | C→S, S→C | Done |
| 10 | MSG_LOGIN | C→S, S→C | Done |
| 12 | MSG_ACTION | C→S, S→C | Done |
| 13 | MSG_CHANGE_OUT | C→S, S→C | Done |
| 14 | MSG_WARP | C→S, S→C | Done |
| 17 | MSG_CHAT | C→S, S→C | Done |
| 18 | MSG_POINT | C→S, S→C | Done |
| 23 | MSG_EMOTE | C→S, S→C | Done |
| 25 | MSG_CHANGE_ACS1 | C→S, S→C | Done |
| 26 | MSG_CHANGE_ACS2 | C→S, S→C | Done |
| 27 | MSG_ROOM_SHOP_INFO | S→C | Done |
| 28 | MSG_SHOP_BUY | C→S, S→C | Done |
| 29 | MSG_SHOP_BUY_FAIL | S→C | Done |
| 30 | MSG_SHOP_STOCK | S→C | Done |
| 31 | MSG_USE_ITEM | C→S, S→C | Done |
| 39 | MSG_DISCARD_ITEM | C→S, S→C | Done |
| 40 | MSG_DISCARDED_ITEM_TAKE | C→S, S→C | Done |
| 43 | MSG_PLAYER_STOP | C→S, S→C | Done |
| 44 | MSG_REQUEST_STATUS | C→S, S→C | Done |
| 45 | MSG_BANK_PROCESS | C→S, S→C | Done |
| 81 | MSG_TOOL_EQUIP | C→S | Done |
| 82 | MSG_TOOL_UNEQUIP | C→S | Done |
| 117 | MSG_PING_REQ | C→S, S→C | Done |
| 47 | MSG_MAILBOX | C→S, S→C | Done |
| 78 | MSG_MAIL_SEND | C→S, S→C | Done |
| 80 | MSG_MAIL_RECEIVER_CHECK | C→S, S→C | Done |
| 19 | MSG_SAVE | C→S, S→C | Done |
| 53 | MSG_SELL_REQ_PRICES | C→S, S→C | Done |
| 54 | MSG_SELL | C→S, S→C | Done |
| 133 | MSG_PLAYER_TYPING | C→S, S→C | Done |
| 32 | MSG_COLLECTIBLE_INFO | S→C | Done |
| 33 | MSG_COLLECTIBLE_TAKE_SELF | C→S | Done |
| 34 | MSG_COLLECTIBLE_TAKEN | S→C | Done |
| 41 | MSG_GET_ITEM | S→C | Done |

## Fixes Applied

### Session 1
- Fixed queued messages not being sent (drain_messages after each handler)
- Fixed visual effects not showing for item user (broadcast to ALL players including sender)
- Fixed outfit/accessory changes not persisting (save to database, lookup actual item ID from slot)
- Fixed acs.sor encryption (re-keyed with correct password)

### Session 2  
- Fixed ping handler to respond correctly (MSG_PING → MSG_PING)
- Added MSG_PING_REQ handler for client latency measurement (F11 debug)
- Added MSG_PLAYER_STOP handler to broadcast when player stops moving
- Added MSG_TOOL_EQUIP/UNEQUIP handlers with ownership validation
- Added full bank system (MSG_REQUEST_STATUS, MSG_BANK_PROCESS)
  - Deposit: wallet → bank
  - Withdraw: bank → wallet
  - Transfer: bank → another player's bank (with atomic transaction rollback)

## Protocol Findings (from 39dll source analysis)

### Message Framing
The client uses **39dll** library for networking. Key finding:
- Each message has a **2-byte length prefix** (little-endian u16)
- The length prefix is **NOT encrypted**
- Only the payload is RC4 encrypted

Wire format:
```
[2 bytes: payload length (NOT encrypted)]
[N bytes: encrypted payload]
```

### Ping System
- **MSG_PING (9)**: Server sends periodically → Client responds → Server responds (keepalive)
- **MSG_PING_REQ (117)**: Client sends when measuring latency → Server echoes back

### Message Format Differences
- **MSG_LOGIN (10)**: Includes version string
  - Format: `[msg_type][version][username][password][mac]`
- **MSG_REGISTER (7)**: Does NOT include version string
  - Format: `[msg_type][username][password][mac]`

### MSG_PLAYER_STOP Format
- **Client → Server**: `[msg_type(2)][x(2)][y(2)]`
- **Server → Client**: `[msg_type(2)][player_id(2)][x(2)][y(2)]`

### RC4 Implementation
The 39dll `bufferencrypt` function uses standard RC4. Our implementation matches exactly.

### Session 3
- Added full mail system (MSG_MAILBOX, MSG_MAIL_SEND, MSG_MAIL_RECEIVER_CHECK)
  - Send mail with text message to other players
  - Attach items from inventory to mail
  - Attach points to mail
  - Receive mailbox (paginated, 5 per page)
  - Claim item/points attachments from mail
  - Username validation before sending
- Added MSG_SAVE handler for manual save points
- Added `auto_save_position` config option (default: false)
  - When false: position/room only saved at manual save points
  - When true: position/room saved on disconnect, warp, and periodically
  - Points are always auto-saved regardless of this setting
- Added graceful shutdown handler (Ctrl+C saves all player data before exit)
- Added full sell system (MSG_SELL_REQ_PRICES, MSG_SELL)
  - Sell outfits, items, accessories, and tools
  - Server-determined sell prices for all item types
  - Multi-item selling (select multiple items to sell at once)
  - Category-based selling (switch between inventory categories)
- Added collectible system (MSG_COLLECTIBLE_INFO, MSG_COLLECTIBLE_TAKE_SELF, MSG_GET_ITEM)
  - Server-side collectible spawn point definitions
  - Automatic respawn after configurable delay (default 5 minutes)
  - Broadcast MSG_COLLECTIBLE_TAKEN to other players in room
  - MSG_COLLECTIBLE_INFO sent on room enter (login/warp)
  - MSG_GET_ITEM to give collected items to player inventory

## Next Steps

1. Parse actual collectible spawn data from game world files
2. Add basic movement validation
3. Implement quest system (MSG_QUEST_NPC_REQ, etc.)
4. Implement clan system (MSG_CLAN_*, etc.)
5. Implement BBS system (MSG_BBS_*, etc.)
