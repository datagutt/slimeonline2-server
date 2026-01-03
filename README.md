# Slime Online 2 - Private Server (Rust)

A Rust implementation of the Slime Online 2 server, compatible with the v0.106 client.

## Table of Contents

- [History](#history)
- [Client Files](#client-files)
- [Features](#features)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Database](#database)
- [Protocol](#protocol)
- [Documentation](#documentation)
- [Tools](#tools)
- [AI Usage](#ai-usage)
- [Contributing](#contributing)
- [License](#license)

## History

Back when I was a kid, I used to play a small, cozy and community-focused platforming MMO called "Slime Online". It went through multiple iterations, and after the game died, the creator published the server and game files for both the first and second iteration of the game.

I backed up (most) of those files, but managed to lose the v2 server files for it. I still had the client and moderator tools, so I set out to create a Rust-powered modern server for it, so I could re-experience the game. The game was made in Game Maker 8.1, a game making engine that can easily be decompiled in modern times.

Fast forward a week or so later, and I managed to get a copy of the server files, making it much easier to finish this project. The original server is Windows-only and uses INI files for storage. This server is usable on any platform, uses modern SQLite databases, and is more configurable and extensible.

With the client frozen in time and for compatibility reasons, this codebase does include weak game protocol encryption and hard-coded RC4 keys that cannot really be changed.

Luckily there seems to be a new iteration of the game in the works by the official creator...

## Client Files

Not currently available. Might upload them later.

## Features

Both the game server and game client contain some unfinished or never-released features due to development ending at an abrupt point.

### Implemented

Some of these features, while *technically* implemented, might not actually work or may contain potentially game-crashing bugs.

| Feature | Status | Notes |
|---------|--------|-------|
| **Authentication** | Complete | Login, registration, bcrypt password hashing |
| **Movement** | Complete | All 13 direction codes, room sync |
| **Chat** | Complete | Room chat, emotes, actions, typing indicator |
| **Appearance** | Complete | Outfit/accessory changes with persistence |
| **Rooms** | Complete | Warping, player tracking, broadcasts |
| **Items** | Complete | Use, discard, pickup with DB persistence |
| **Collectibles** | Complete | Spawn points, respawn timers, item collection |
| **Shops** | Complete | Buy items, daily stock restock (on day change) |
| **Selling** | Complete | Sell items/outfits/accessories/tools |
| **Banking** | Complete | Deposit, withdraw, transfer to other players |
| **Mail** | Complete | Send/receive mail with item/point attachments |
| **BBS** | Complete | Post, read, report messages |
| **Clans** | Complete | Create, dissolve, invite, kick, leave, info |
| **Quests** | Complete | Begin, cancel, step, reward (Quest 1 implemented) |
| **Dropped Items** | Complete | DB persistence, 3-min expiration, broadcast |
| **Top Points** | Complete | Leaderboard sign in city rooms (42, 126) |
| **Tools** | Complete | Equip/unequip with persistence |
| **Save Points** | Complete | Manual save at save point NPCs |
| **Planting System** | Complete | Plant trees, add pinwheels/fairies, harvest fruit |
| **Upgrader System** | Complete | Town upgrader with points investment |
| **Storage Extension** | Complete | Extended item storage with pages |
| **Cannon System** | Complete | Enter, move, set power, shoot cannons |
| **Racing System** | Complete | Race info, start, checkpoints, end, mount/dismount |
| **Vending Machines** | Complete | Gum and soda machines with random items |
| **One-Time Items** | Complete | Collectible one-time items per character |
| **Music Changer** | Complete | Change room background music |

### Not Yet Implemented

| Feature | Priority | Messages |
|---------|----------|----------|
| **Building System** | Low | 4 messages (103-106) |

### Might Implement

- Automatic shop item rotation (new items for daily restock)
- Mod tools (SO2 ones are weak)
- Live player map in browser

### Untested / Needs Verification

- Clan functionality
- Quests
- Collectible evolution (mushroom transformation over time)
- Full multi-player stress testing

## Quick Start

### Prerequisites

- Rust 1.70+
- SQLite3

### Build & Run

```bash
# Clone and build
cd rust_server
cargo build --release

# Run (creates database automatically)
cargo run --release

# Server listens on 0.0.0.0:5555
```

### Configuration

Configuration files are in `config/`:

| File | Description |
|------|-------------|
| `server.toml` | Server settings (port, database, MOTD) |
| `game.toml` | Game rules, spawn point, limits |
| `prices.toml` | Item/outfit/accessory/tool prices |
| `shops.toml` | Shop inventories per room |
| `collectibles.toml` | Collectible spawn points per room |
| `plants.toml` | Plant growth configuration |
| `clans.toml` | Clan system settings |

## Architecture

```
src/
├── main.rs              # Entry point, TCP listener, background tasks
├── config/              # Configuration loading (TOML)
├── crypto.rs            # RC4 encryption
├── protocol/            # Binary message parsing/writing
├── handlers/            # Message handlers by category
│   ├── auth.rs          # Login/register
│   ├── movement.rs      # Player movement
│   ├── chat.rs          # Chat, emotes, typing
│   ├── warp.rs          # Room changes
│   ├── items/           # Item use, discard, pickup
│   ├── shop/            # Buy, sell
│   ├── bank.rs          # Banking operations
│   ├── mail.rs          # Mail system
│   ├── bbs.rs           # Bulletin board
│   ├── clan.rs          # Clan system
│   ├── quest.rs         # Quest system
│   └── collectibles.rs  # Collectible spawns
├── game/                # Game state, rooms, sessions
└── db/                  # Database operations
    ├── accounts.rs
    ├── characters.rs
    ├── clans.rs
    ├── mail.rs
    ├── bbs.rs
    └── runtime_state.rs # Collectibles, plants, shops, ground items

migrations/              # SQLite migrations
config/                  # TOML configuration files
sor_tool/               # SOR archive encryption tool
```

## Database

SQLite database with auto-migrations.

| Table | Description |
|-------|-------------|
| `accounts` | User authentication |
| `characters` | Player data (position, points, appearance) |
| `inventories` | Equipment slots (emotes, outfits, accessories, items, tools) |
| `clans` | Clan data |
| `clan_members` | Clan membership |
| `mail` | Player mail |
| `bbs_posts` | Bulletin board posts |
| `collectible_state` | Collectible availability/respawn |
| `plant_state` | Plant growth progress |
| `shop_stock` | Limited shop item stock |
| `ground_items` | Dropped items on ground |
| `server_state` | Key-value store (restock date, etc.) |
| `quest_progress` | Completed quests per character |
| `bans` | IP/MAC/account bans |

## Protocol

The game protocol security for this old game was not entirely great, but it is a 15 year old game at this point. I am not really interested in updating the game client, as that is outside the scope of the project.

- **Port:** 5555
- **Encryption:** RC4 with hardcoded keys
- **Client Version:** 0.106
- **Message Format:** 2-byte length prefix + encrypted payload

```rust
// Server decrypts with:
const DECRYPT_KEY: &[u8] = b"retrtz7jmijb5467n47";
// Server encrypts with:
const ENCRYPT_KEY: &[u8] = b"t54gz65u74njb6zg6";
```

## Documentation

Detailed documentation is in `docs/`:

| Directory | Description |
|-----------|-------------|
| `docs/protocol/` | Network protocol, message formats |
| `docs/architecture/` | Server architecture design |
| `docs/database/` | Database schema |
| `docs/game-systems/` | Game mechanics |
| `docs/security/` | Validation, anti-cheat |

## Tools

### SOR Tool

SOR is an archive format used for the game client resource files. This tool lets you decrypt, re-encrypt and re-key game files. The SO2 client usually had the encryption key changed every client version. I at one point had to re-key an old game file, as I was missing the `.sor` file belonging to the latest client.

```bash
cd sor_tool
cargo run -- list archive.sor
cargo run -- extract archive.sor password output_dir
cargo run -- create input_dir password output.sor
cargo run -- rekey archive.sor old_password new_password output.sor
```

## AI Usage

Claude Opus 4.5 was used to generate the initial protocol docs, due to its ability to rapidly read and analyze huge codebases. I initially had it read the decompiled client code, mod tools, client data files and 39DLL C++ Source Code. After gaining access to the decompiled server source code, Claude attempted to update the docs based on the new knowledge.

A lot of the Rust code is also generated, but reviewed and adjusted by hand.

## Contributing

While I do not expect a lot of contributions due to the obscurity of the project, any new features or bug fixes are welcome. Especially the protocol documentation could use some love, as I am pretty sure it still has some LLM hallucinations.

## License

MIT License.

This is a legacy game preservation project and is not intended to compete with any new game projects by the original Slime Online game owner.
