# Client Limits and Constraints

**Document Status:** Complete  
**Last Updated:** 2024-12-20  
**Source:** Decompiled client v0.106

This document describes the hard-coded limits in the Slime Online 2 client that the server **MUST** respect.

## Constants from Client

```rust
// Version
pub const GAME_VERSION: &str = "0.106";

// Points/Currency
pub const MAX_POINTS: u32 = 10_000_000;  // Maximum slime points a player can have
pub const PRICE_SODA: u16 = 20;          // Cost of soda from machine
pub const PRICE_GUM: u16 = 10;           // Cost of gum from machine
```

## Data Type Limits

The client reads specific data types for each field. The server MUST send data within these bounds:

### Player/Entity IDs
| Field | Read Type | Range | Notes |
|-------|-----------|-------|-------|
| player_id | `readushort()` | 0-65535 | Unique per session |
| instance_id | `readushort()` | 0-65535 | For discarded items |

### Positions
| Field | Read Type | Range | Notes |
|-------|-----------|-------|-------|
| x position | `readushort()` | 0-65535 | Room coordinates |
| y position | `readushort()` | 0-65535 | Room coordinates |
| room_id | `readushort()` | 0-65535 | Room index |

### Items/Inventory
| Field | Read Type | Range | Notes |
|-------|-----------|-------|-------|
| item_id | `readushort()` | 0-65535 | Item type ID |
| slot | `readbyte()` | 0-255 | Inventory slot |
| quantity | `readushort()` | 0-65535 | Stack size |
| tool_id | `readbyte()` | 0-255 | Tool type |

### Collectibles
| Field | Read Type | Range | Notes |
|-------|-----------|-------|-------|
| collectible_count | `readbyte()` | 0-255 | Max collectibles per room message |
| collectible_slot | `readbyte()` | 0-255 | Slot/index in room |
| collectible_id | `readushort()` | 0-65535 | Collectible type |
| x | `readushort()` | 0-65535 | Position |
| y | `readushort()` | 0-65535 | Position |

### Time
| Field | Read Type | Range | Notes |
|-------|-----------|-------|-------|
| server_time | `readuint()` | 0-4294967295 | Milliseconds |
| day | `readbyte()` | 1-7 | Day of week (Sun=1) |
| hour | `readbyte()` | 0-23 | Hour |
| minute | `readbyte()` | 0-59 | Minute |

## Inventory Slot Limits

```rust
// These are FIXED in the client - arrays are indexed 1-9
pub const OUTFIT_SLOTS: usize = 9;    // global.sl_outfits[1..9]
pub const ITEM_SLOTS: usize = 9;      // global.sl_items[1..9]
pub const ACS_SLOTS: usize = 9;       // global.sl_acs[1..9]
pub const TOOL_SLOTS: usize = 9;      // global.sl_tools[1..9]
pub const EMOTE_SLOTS: usize = 5;     // global.sl_emotes[1..5]
```

**IMPORTANT:** Inventory arrays are 1-indexed in the client! Slots are 1-9, not 0-8.

## Clan Limits

```rust
pub const CLAN_MIN_MEMBERS: u8 = 3;   // Default member slots
pub const CLAN_MAX_MEMBERS: u8 = 10;  // Maximum possible member slots
```

The `clan_maxmembers` is sent as `readbyte()` and represents how many slots are unlocked.
Members array is indexed 1-10: `global.clan_members[1..10]`

## Storage Limits

```rust
pub const STORAGE_MAX_PAGES: u16 = 20;  // Maximum storage pages (per category)
pub const STORAGE_SLOTS_PER_PAGE: u8 = 20;  // Slots per page
```

Storage page is read as `readushort()`, individual slots as iteration `for (_i=1;_i<=20;_i+=1)`

## Chat/Text Limits

Based on client input fields and network reads:

```rust
pub const USERNAME_MAX_LENGTH: usize = 20;  // Approximate
pub const CHAT_MAX_LENGTH: usize = 100;     // Approximate  
pub const CLAN_NAME_MAX_LENGTH: usize = 20; // Approximate
```

## Room/Map Limits

```rust
// Room indices start at 0 in the project file
// Game rooms start after menu rooms (index ~9+)
pub const FIRST_GAME_ROOM: u16 = 9;  // rm_frei_0 (placeholder)

// Slime points per room (obj_slimepoint instances)
// global.rm_points starts at 1 and increments per spawn point
// Maximum observed: ~85 spawn points per room
```

## Collectible System

The server sends collectibles via `MSG_COLLECTIBLE_INFO`:

```
Format:
- count: byte (0-255 collectibles)
- For each collectible:
  - slot: byte (unique per room, 0-255)
  - item_id: ushort (collectible type)
  - x: ushort (position)
  - y: ushort (position)
```

**Key insight:** Collectible positions are 100% server-determined. The client just renders them where told.

The `obj_slimepoint` objects in room files are NOT collectible spawn points - they are currency/XP points that the client handles locally.

## Message Size Considerations

Based on 39dll buffer operations:
- Maximum practical message size: ~64KB (limited by ushort length prefix)
- Recommended: Keep individual messages under 4KB for best performance

## Sprite/Visual Limits

| Item Type | Sprite | Max Frames | Notes |
|-----------|--------|------------|-------|
| Outfits | dynamic load | ~100+ types | Loaded from outfits.sor |
| Accessories | dynamic load | ~100+ types | Loaded from acs.sor |
| Items | spr_items_index | 256 frames | Fixed sprite sheet |
| Collectibles | spr_collectibles_index | 256 frames | Fixed sprite sheet |
| Tools | spr_tools_index | 256 frames | Fixed sprite sheet |
| Emotes | dynamic load | ~50+ types | Loaded from emotes.sor |

## Network Protocol Limits

```rust
// Encryption keys (hardcoded - DO NOT CHANGE)
pub const DECRYPT_KEY: &[u8] = b"t54gz65u74njb6zg6";
pub const ENCRYPT_KEY: &[u8] = b"retrtz7jmijb5467n47";

// Default port
pub const DEFAULT_PORT: u16 = 5555;

// Ping interval
pub const PING_INTERVAL_FRAMES: u32 = 1800;  // ~30 seconds at 60fps
```

## Summary Table

| Constraint | Limit | Data Type |
|------------|-------|-----------|
| Max points | 10,000,000 | u32 |
| Inventory slots (each type) | 9 | - |
| Emote slots | 5 | - |
| Clan members | 3-10 | u8 |
| Storage pages | 20 | u16 |
| Storage slots per page | 20 | - |
| Collectibles per message | 255 | u8 |
| Player ID | 65535 | u16 |
| Room ID | 65535 | u16 |
| Position (x,y) | 65535 | u16 |
| Item ID | 65535 | u16 |

## Important Notes for Server Implementation

1. **Inventory is 1-indexed** - Always use slots 1-9, never slot 0
2. **Collectibles are server-authoritative** - Client just renders what server sends
3. **Points have hard cap** - Don't allow points > 10,000,000
4. **Version must match exactly** - Client checks "0.106" string match
5. **All positions use ushort** - Maximum coordinate is 65535
6. **Slot IDs use byte** - Maximum 255 unique collectibles per room
