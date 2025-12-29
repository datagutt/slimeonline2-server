# Slime Online 2 Server - Implementation Progress

## Overview

This document tracks the implementation progress of the Slime Online 2 private server.

**Last Updated:** 2024-12-29

## Implementation Status Summary

| Category | Status | Messages Implemented |
|----------|--------|---------------------|
| Authentication | Complete | 2/2 |
| Player Management | Complete | 6/6 |
| Movement | Complete | 2/2 |
| Communication | Complete | 4/4 |
| Items & Inventory | Complete | 6/6 |
| Shop & Economy | Complete | 5/5 |
| Banking | Complete | 2/2 |
| Mail | Complete | 4/4 |
| BBS | Complete | 8/8 |
| Clans | Complete | 6/6 |
| Quests | Complete | 10/10 |
| Collectibles | Complete | 3/3 |
| Tools | Complete | 2/2 |
| Utility | Complete | 5/5 |
| **Planting** | **Not Started** | 0/9 |
| **Storage** | **Not Started** | 0/3 |
| **Building** | **Not Started** | 0/4 |
| **Cannon** | **Not Started** | 0/4 |
| **Racing** | **Not Started** | 0/6 |
| **Upgrader** | **Not Started** | 0/5 |
| **Music** | **Not Started** | 0/2 |
| **One-Time Items** | **Not Started** | 0/3 |

**Total: ~65 messages implemented out of ~141**

---

## Completed Features

### Phase 1: Foundation

| Task | Status | File(s) |
|------|--------|---------|
| Project setup with Cargo.toml | Done | `Cargo.toml` |
| Directory structure | Done | `src/` |
| RC4 encryption/decryption | Done | `src/crypto.rs` |
| MessageReader (binary parsing) | Done | `src/protocol/reader.rs` |
| MessageWriter (binary serialization) | Done | `src/protocol/writer.rs` |
| Message structures | Done | `src/protocol/messages.rs` |
| Game constants (all 141 message types) | Done | `src/constants.rs`, `src/protocol/types.rs` |
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
| Configuration system (TOML) | Done | `src/config/mod.rs`, `config/*.toml` |

### Phase 2: Core Gameplay

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
| MSG_SAVE handler | Done | `src/handlers/connection.rs` |
| Room player tracking | Done | `src/game/mod.rs` |
| Player broadcast to room | Done | Various handlers |
| New player notification | Done | `src/handlers/auth.rs` |
| Player leave notification | Done | `src/handlers/connection.rs` |

### Phase 3: Items & Economy

| Task | Status | File(s) |
|------|--------|---------|
| Item database (61 items) | Done | `src/handlers/items/database.rs` |
| MSG_USE_ITEM handler | Done | `src/handlers/items/use_item.rs` |
| MSG_DISCARD_ITEM handler | Done | `src/handlers/items/discard.rs` |
| MSG_DISCARDED_ITEM_TAKE handler | Done | `src/handlers/items/pickup.rs` |
| Dropped item DB persistence | Done | `src/db/runtime_state.rs` |
| Dropped item expiration (3 min) | Done | `src/main.rs` |
| MSG_SHOP_BUY handler | Done | `src/handlers/shop/buy.rs` |
| MSG_ROOM_SHOP_INFO sender | Done | `src/handlers/shop/buy.rs` |
| Shop info on room enter | Done | `src/handlers/warp.rs` |
| Shop stock persistence | Done | `src/db/runtime_state.rs` |
| Daily shop restock (day change only) | Done | `src/main.rs` |
| MSG_REQUEST_STATUS handler | Done | `src/handlers/bank.rs` |
| MSG_BANK_PROCESS handler | Done | `src/handlers/bank.rs` |
| Bank transfer with transactions | Done | `src/db/characters.rs` |
| MSG_TOOL_EQUIP handler | Done | `src/handlers/connection.rs` |
| MSG_TOOL_UNEQUIP handler | Done | `src/handlers/connection.rs` |
| MSG_SELL_REQ_PRICES handler | Done | `src/handlers/shop/sell.rs` |
| MSG_SELL handler | Done | `src/handlers/shop/sell.rs` |
| Collectible system | Done | `src/handlers/collectibles.rs` |
| Collectible respawn timer | Done | `src/main.rs`, `src/db/runtime_state.rs` |

### Phase 4: Social Features

| Task | Status | File(s) |
|------|--------|---------|
| Mail database schema | Done | `migrations/20240101000007_create_mail.sql` |
| MSG_MAILBOX handler | Done | `src/handlers/mail.rs` |
| MSG_MAIL_SEND handler | Done | `src/handlers/mail.rs` |
| MSG_MAIL_RECEIVER_CHECK handler | Done | `src/handlers/mail.rs` |
| MSG_MAILPAPER_REQ handler | Done | `src/handlers/connection.rs` |
| Mail item/points attachments | Done | `src/handlers/mail.rs` |
| BBS database schema | Done | `migrations/20240101000008_create_bbs.sql` |
| All 8 BBS message handlers | Done | `src/handlers/bbs.rs` |
| Clan database schema | Done | `migrations/20240101000004_create_clans.sql` |
| MSG_CLAN_CREATE handler | Done | `src/handlers/clan.rs` |
| MSG_CLAN_DISSOLVE handler | Done | `src/handlers/clan.rs` |
| MSG_CLAN_INVITE handler | Done | `src/handlers/clan.rs` |
| MSG_CLAN_LEAVE handler | Done | `src/handlers/clan.rs` |
| MSG_CLAN_INFO handler | Done | `src/handlers/clan.rs` |
| MSG_CLAN_ADMIN handler | Done | `src/handlers/clan.rs` |

### Phase 5: Game Systems

| Task | Status | File(s) |
|------|--------|---------|
| Quest database schema | Done | `migrations/20240101000011_create_quest_progress.sql` |
| MSG_QUEST_BEGIN handler | Done | `src/handlers/quest.rs` |
| MSG_QUEST_CLEAR handler | Done | `src/handlers/quest.rs` |
| MSG_QUEST_STEP_INC handler | Done | `src/handlers/quest.rs` |
| MSG_QUEST_CANCEL handler | Done | `src/handlers/quest.rs` |
| MSG_QUEST_NPC_REQ handler | Done | `src/handlers/quest.rs` |
| MSG_QUEST_REWARD handler | Done | `src/handlers/quest.rs` |
| MSG_QUEST_VAR_* handlers | Done | `src/handlers/quest.rs` |
| Quest 1 reward logic | Done | `src/handlers/quest.rs` |
| MSG_GET_TOP_POINTS | Done | `src/handlers/warp.rs`, `src/handlers/connection.rs` |
| Top points query | Done | `src/db/characters.rs` |

---

## Not Yet Implemented

### Planting System (9 messages: 63-70, 94)

| Message | ID | Description |
|---------|-----|-------------|
| MSG_PLANT_SPOT_FREE | 63 | Server → Client: spot is available |
| MSG_PLANT_SPOT_USED | 64 | Server → Client: spot has plant |
| MSG_PLANT_DIE | 65 | Server → Client: plant died |
| MSG_PLANT_GROW | 66 | Server → Client: plant grew a stage |
| MSG_PLANT_ADD_PINWHEEL | 67 | Client → Server: add pinwheel to plant |
| MSG_PLANT_ADD_FAIRY | 68 | Client → Server: add fairy to plant |
| MSG_PLANT_GET_FRUIT | 69 | Client → Server: harvest fruit |
| MSG_PLANT_HAS_FRUIT | 70 | Server → Client: plant has fruit ready |
| MSG_TREE_PLANTED_INC | 94 | Server → Client: increment trees planted stat |

### Storage Extension (3 messages: 56-58)

| Message | ID | Description |
|---------|-----|-------------|
| MSG_STORAGE_REQ | 56 | Client → Server: open storage |
| MSG_STORAGE_PAGES | 57 | Server → Client: storage page data |
| MSG_STORAGE_MOVE | 58 | Client → Server: move item to/from storage |

### Building System (4 messages: 103-106)

| Message | ID | Description |
|---------|-----|-------------|
| MSG_BUILD_SPOT_FREE | 103 | Server → Client: build spot available |
| MSG_BUILD_SPOT_USED | 104 | Server → Client: build spot occupied |
| MSG_BUILD_SPOT_BECOME_FREE | 105 | Server → Client: build spot freed |
| MSG_OBJECTS_BUILT_INC | 106 | Server → Client: increment objects built stat |

### Cannon System (4 messages: 98-101)

| Message | ID | Description |
|---------|-----|-------------|
| MSG_CANNON_ENTER | 98 | Client → Server: enter cannon |
| MSG_CANNON_MOVE | 99 | Client → Server: aim cannon |
| MSG_CANNON_SET_POWER | 100 | Client → Server: set power |
| MSG_CANNON_SHOOT | 101 | Client → Server: fire cannon |

### Racing System (6 messages: 120-125)

| Message | ID | Description |
|---------|-----|-------------|
| MSG_RACE_INFO | 120 | Server → Client: race info |
| MSG_RACE_START | 121 | Server → Client: race started |
| MSG_RACE_CHECKPOINT | 122 | Client → Server: hit checkpoint |
| MSG_RACE_END | 123 | Server → Client: race ended |
| MSG_MOVE_GET_ON | 124 | Client → Server: mount vehicle |
| MSG_MOVE_GET_OFF | 125 | Client → Server: dismount vehicle |

### Other Systems

- Upgrader System (5 messages: 108-112)
- Music Changer (2 messages: 95-96)
- One-Time Items (3 messages: 35-37)

---

## Recent Changes

### Session - December 2024 (Latest)

- Implemented full Clan System (6 messages: 126-131)
  - Create clan with Proof of Nature + Proof of Earth + 10,000 SP
  - Dissolve clan (leader only)
  - Invite players with 15s cooldown
  - Accept/decline invites
  - Leave clan
  - Kick members (leader only)
  - Get clan info (5 sub-types)
  - Admin actions (colors, info text, news)
  
- Implemented full Quest System (10 messages: 83-92)
  - Begin, cancel, step increment
  - Quest variable check/set/increment
  - NPC request (check if cleared)
  - Quest reward (with Quest 1 "Lazy Coolness" logic)
  
- Fixed shop restock behavior
  - Shops do NOT restock on server restart
  - Only restock when calendar day actually changes
  - Persisted last_restock_date in server_state table

- Implemented dropped items DB persistence
  - Items saved to ground_items table
  - 3-minute expiration with client notification
  - Survives server restart
  - Removed in-memory dropped items (now DB-only)

- Implemented Top Player Points sign
  - Query-based (no separate table entry needed)
  - Sent when entering rooms 42 or 126
  - Broadcast to city rooms when new top on save

- Fixed mail paper availability from config

---

## Architecture

```
src/
├── main.rs              # Entry point, TCP listener, background tasks
├── config/              # Configuration loading (TOML)
├── crypto.rs            # RC4 encryption
├── protocol/            # Binary message parsing/writing
├── handlers/            # Message handlers by category
├── game/                # Game state, rooms, sessions
└── db/                  # Database operations

migrations/              # SQLite migrations (12 files)
config/                  # TOML configuration files (7 files)
```

## Database Schema

12 migration files creating:
- accounts, characters, inventories
- clans, clan_members
- bans, shop_stock
- mail, bbs_posts
- collectible_state, plant_state, ground_items
- server_state, quest_progress

## Next Steps

1. Implement Planting System (9 messages)
2. Implement Storage Extension (3 messages)
3. Add more quest reward logic (Quest 2+)
4. Implement collectible evolution (mushroom transformation)
5. Add remaining low-priority systems
