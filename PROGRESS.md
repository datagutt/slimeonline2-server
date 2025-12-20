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
| Room player tracking | Done | `src/game/mod.rs` |
| Player broadcast to room | Done | Various handlers |
| New player notification | Done | `src/handlers/auth.rs` |
| Player leave notification | Done | `src/handlers/connection.rs` |
| Points persistence | Done | `src/handlers/gameplay.rs`, `src/db/characters.rs` |
| Room/position persistence | Done | `src/handlers/warp.rs`, `src/db/characters.rs` |

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

### Pending

| Task | Status | Notes |
|------|--------|-------|
| MSG_GET_ITEM handler | Pending | Receive item from world |
| MSG_SELL handler | Pending | Sell items in shop |
| Bank system (MSG_BANK_*) | Pending | Deposit, withdraw, transfer |

## Phase 4: Social Features (Week 9-11)

### Pending

| Task | Status | Notes |
|------|--------|-------|
| Clan creation/dissolution | Pending | MSG_CLAN_CREATE, MSG_CLAN_DISSOLVE |
| Clan invites/kicks | Pending | MSG_CLAN_INVITE, MSG_CLAN_LEAVE |
| Mail system | Pending | MSG_MAILBOX, MSG_MAIL_SEND |
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
│   ├── connection.rs    # Connection lifecycle and message routing
│   ├── auth.rs          # Login/register handlers
│   ├── movement.rs      # Movement message handler
│   ├── chat.rs          # Chat, emote, action, typing handlers
│   ├── appearance.rs    # Outfit and accessory change handlers
│   ├── gameplay.rs      # Points collection and gameplay handlers
│   ├── warp.rs          # Room change/warp handler
│   ├── items/           # Item system handlers
│   │   ├── mod.rs
│   │   ├── database.rs  # 61 items from db_items.gml
│   │   ├── use_item.rs  # MSG_USE_ITEM
│   │   ├── discard.rs   # MSG_DISCARD_ITEM
│   │   └── pickup.rs    # MSG_DISCARDED_ITEM_TAKE
│   └── shop/            # Shop system handlers
│       ├── mod.rs
│       └── buy.rs       # MSG_SHOP_BUY, MSG_ROOM_SHOP_INFO
├── game/
│   └── mod.rs           # Game state, rooms, sessions, dropped items
└── db/
    ├── mod.rs           # Database pool and migrations
    ├── accounts.rs      # Account queries
    └── characters.rs    # Character queries

migrations/
├── 20240101000001_create_accounts.sql
├── 20240101000002_create_characters.sql
├── 20240101000003_create_inventories.sql
├── 20240101000004_create_clans.sql
├── 20240101000005_create_bans.sql
└── 20240101000006_create_shops.sql   # Dynamic shop inventory with seed data
```

## Database Schema

- **accounts**: User authentication (username, password_hash, mac_address, ban status)
- **characters**: Player data (position, appearance, points, quest state)
- **inventories**: Equipment and items (emotes, outfits, accessories, items, tools)
- **clans**: Clan information (name, leader, colors, level)
- **bans**: IP/MAC/account bans
- **shop_items**: Dynamic shop inventory (room_id, slot_id, category, item_id, price, stock)

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
| 133 | MSG_PLAYER_TYPING | C→S, S→C | Done |
| 27 | MSG_ROOM_SHOP_INFO | S→C | Done |
| 28 | MSG_SHOP_BUY | C→S, S→C | Done |
| 29 | MSG_SHOP_BUY_FAIL | S→C | Done |
| 30 | MSG_SHOP_STOCK | S→C | Done |
| 31 | MSG_USE_ITEM | C→S | Done |
| 39 | MSG_DISCARD_ITEM | C→S, S→C | Done |
| 40 | MSG_DISCARDED_ITEM_TAKE | C→S, S→C | Done |

## Known Issues

1. No movement validation (position/speed checks)
2. Appearance changes use slot index instead of actual item ID lookup

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

### Message Format Differences
- **MSG_LOGIN (10)**: Includes version string
  - Format: `[msg_type][version][username][password][mac]`
- **MSG_REGISTER (7)**: Does NOT include version string
  - Format: `[msg_type][username][password][mac]`

### MSG_WARP Format
- **Client → Server**: `[msg_type(2)][room_id(2)][x(2)][y(2)]`
- **Server → Client (enter)**: `[msg_type(2)][player_id(2)][case=1(1)][x(2)][y(2)]`
- **Server → Client (leave)**: `[msg_type(2)][player_id(2)][case=2(1)]`

### MSG_POINT Format
- **Client → Server**: `[msg_type(2)][point_index(1)]`
- Points are incremented server-side and persisted to database

### RC4 Implementation
The 39dll `bufferencrypt` function uses standard RC4. Our implementation matches exactly.

## Next Steps

1. Implement bank system (MSG_BANK_PROCESS)
2. Implement sell handler (MSG_SELL)
3. Add basic movement validation
4. Implement clan system
5. Implement mail system
