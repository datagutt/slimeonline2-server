# Upgrader System

## Overview

The Upgrader System is a **community investment** system in Slime Online 2. Players collectively invest points to unlock new features for their town/city. The upgrader machine is located in cities (New City - room 42, Old City - room 121), and all players can contribute points towards shared upgrades that benefit everyone.

**Important:** This is NOT about upgrading items or tools. It's a town-wide feature unlock system where player contributions are pooled together.

## Message IDs

| Message ID | Constant | Direction | Purpose |
|------------|----------|-----------|---------|
| 108 | `MSG_UPGRADER_GET` | C->S, S->C | Request/receive upgrade slot information |
| 109 | `MSG_UPGRADER_POINTS` | C->S, S->C | Request/receive points needed for a slot |
| 110 | `MSG_UPGRADER_INVEST` | C->S, S->C | Invest points into an upgrade |
| 111 | `MSG_UPGRADE_APPEAR` | S->C | (Unused in current code) |
| 112 | `MSG_UNLOCKABLE_EXISTS` | S->C | Notify client an unlockable is available |

## Upgrader Categories

| Category ID | Name | Purpose |
|-------------|------|---------|
| 0 | Other Upgrades | Unlock bubblegum/soda machines, decorations, music changers |
| 1 | Warp Center Upgrades | Unlock new warp destinations |
| 2 | Outfit Shop Upgrades | Unlock new outfits, increase stock |
| 3 | Acs (Accessory) Shop Upgrades | Unlock new accessories, increase stock |
| 4 | Item Shop Upgrades | Unlock new items, increase stock |

## Message Format Details

### MSG_UPGRADER_GET (108)

**Client -> Server (Request):**
```
u16: message_id (108)
u16: town_id          // City room ID (42=New City, 121=Old City)
u8:  category         // Category (0-4)
u8:  page             // Page number (0-based, 4 slots per page)
```

**Server -> Client (Response):**
```
u16: message_id (108)
// For each of 4 slots on the page:
string: slot_name     // Name of the upgrade (empty if slot doesn't exist)
u8:     percentage    // Progress percentage:
                      //   0 = slot doesn't exist
                      //   1-99 = progress percent
                      //   100 = completed
                      //   250 = locked (upgrade exists but not yet unlocked)
// Repeat for all 4 slots (t1, p1, t2, p2, t3, p3, t4, p4)
u8: has_more          // 1 = more pages exist, 0 = no more pages
```

### MSG_UPGRADER_POINTS (109)

**Client -> Server (Request):**
```
u16: message_id (109)
u16: town_id          // City room ID
u8:  category         // Category (0-4)
u8:  slot_id          // Slot ID (1-based)
```

**Server -> Client (Response):**
```
u16: message_id (109)
// Either:
u8: 123               // Error: slot doesn't exist
// Or:
u16: points_needed    // Points still needed (divided by 10!)
```

**Note:** The server divides the remaining points by 10 before sending. The client multiplies by 10 to get the real value.

### MSG_UPGRADER_INVEST (110)

**Client -> Server (Request):**
```
u16: message_id (110)
u16: town_id          // City room ID
u8:  category         // Category (0-4)
u8:  slot_id          // Slot ID (1-based)
u8:  invest_code      // Investment amount:
                      //   1 = 100 points
                      //   2 = 500 points
                      //   3 = 1000 points
                      //   4 = 5000 points
```

**Server -> Client (Response):**
On success, sends two messages:

1. Points deduction:
```
u16: message_id (53)  // MSG_POINTS_DEC
u16: points_deducted  // Amount deducted
```

2. Confirmation:
```
u16: message_id (110) // MSG_UPGRADER_INVEST
// No additional data - just signals completion
```

### MSG_UNLOCKABLE_EXISTS (112)

**Server -> Client:**
```
u16: message_id (112)
u8:  unlockable_id    // ID of the unlockable in the room
```

Sent when entering a room or when an upgrade unlocks something. The client uses this to show/enable objects that were previously hidden.

## Upgrader Configuration

### File Structure

Upgrader state is stored per-town. In the original server, this was INI files in `srvr_upgrader/`:
- `42.upg` - New City upgrades
- `121.upg` - Old City upgrades

For the Rust server, this is stored in the database with a TOML configuration file for initial setup.

### Categories (INI Sections)

- `[Other]` - Miscellaneous upgrades
- `[Warp Center]` - Warp destination unlocks
- `[Outfit Shop]` - Outfit shop upgrades
- `[Acs Shop]` - Accessory shop upgrades
- `[Item Shop]` - Item shop upgrades

### Upgrade Slot Fields

Each slot is prefixed with its number (e.g., `1name`, `2name`):

| Field | Description |
|-------|-------------|
| `{n}name` | Display name of the upgrade |
| `{n}need` | Total points required to complete |
| `{n}paid` | Points already invested |
| `{n}unlocked` | 1 = visible to players, 0 = hidden |
| `{n}option` | Action type when completed |
| `{n}other1` - `{n}other5` | Parameters for the action |

### Unlock Chaining

Upgrades can unlock other upgrades when completed:

| Field | Description |
|-------|-------------|
| `{n}unlockupgrade{m}` | Slot ID to unlock when this completes |
| `{n}unlockcategory{m}` | Category of the slot to unlock |

## Upgrade Option Types

### "Other" Category (option field)

| Option | Action | Parameters |
|--------|--------|------------|
| 1 | Unlock an unlockable object | `other1`=room_id, `other2`=unlockable_id |
| 2 | Unlock music for Music Changer | `other1`=room_id, `other2`=song_slot, `other3`=1 (day) or 2 (night) |

### "Warp Center" Category

No option field. Uses special fields:
- `room` - Room ID where the warp center is located
- `{n}warpslot` - Slot number in the warp center
- `{n}warpcat` - Warp category: 1=City, 2=Fields, 3=Dungeon

### "Outfit/Acs/Item Shop" Categories (option field)

| Option | Action | Parameters |
|--------|--------|------------|
| 1 | Unlock new item in shop | `other1`=shop_room_id, `other2`=slot_id |
| 2 | Increase daily stock | `other1`=shop_room_id, `other2`=stock_increase |

## Server Handler Logic

### MSG_UPGRADER_GET Handler

1. Read town_id, category, page from client
2. Load upgrader state for the town
3. Determine category key based on category ID
4. For slots `(page*4)+1` through `(page*4)+4`:
   - If slot exists and is unlocked: send name + percentage
   - If slot exists but not unlocked: send empty name + 250
   - If slot doesn't exist: send empty name + 0
5. Check if slot `(page*4)+5` exists and is unlocked for "has_more" flag

### MSG_UPGRADER_POINTS Handler

1. Read town_id, category, slot_id
2. Load upgrader state
3. If slot doesn't exist: send 123 (error code)
4. Otherwise: calculate `(need - paid) / 10` and send as u16

### MSG_UPGRADER_INVEST Handler

1. Read town_id, category, slot_id, invest_code
2. Convert invest_code to points:
   - 1 = 100 points
   - 2 = 500 points
   - 3 = 1000 points
   - 4 = 5000 points
3. Validate player has enough points
4. Add points to the slot's `paid` value
5. Deduct points from player and save
6. If `paid >= need`: apply the upgrade effect
7. Send `MSG_POINTS_DEC` with amount deducted
8. Send `MSG_UPGRADER_INVEST` confirmation

### Upgrade Completion Effects

When an upgrade is fully funded:

**Warp Center:** Write unlock flag to room's warp center configuration

**Outfit/Acs/Item Shop:**
- Option 1: Mark shop slot as available, broadcast `MSG_ROOM_SHOP_INFO` to players in shop room
- Option 2: Increase stock for all items in shop, broadcast `MSG_SHOP_STOCK`

**Other:**
- Option 1: Mark unlockable as available, broadcast `MSG_UNLOCKABLE_EXISTS` to players in room
- Option 2: Mark music slot as unlocked in room configuration

**Unlock Chaining:** Check for `unlockupgrade` entries and set their `unlocked` flag to 1

## Client UI Flow

1. Player approaches `obj_upgrader` and presses action key
2. GUI opens with "Processing" screen
3. Client sends `MSG_UPGRADER_GET` to fetch current page data
4. Server responds with slot names and percentages
5. Client displays main UI with progress bars (0-100% shown as colored bars)
6. When player clicks "Invest" on a slot:
   - Client sends `MSG_UPGRADER_POINTS` to get remaining points needed
   - Server responds with `points / 10`
   - Client shows invest screen with 100/500/1000/5000 buttons
7. Player clicks an amount button:
   - Client validates they have enough points and upgrade needs that many
   - Client sends `MSG_UPGRADER_INVEST`
   - Server deducts points and applies upgrade if complete
   - Client returns to main screen

## Database Schema

```sql
-- Upgrader state per town
CREATE TABLE upgrader_state (
    town_id INTEGER NOT NULL,
    category TEXT NOT NULL,
    slot_id INTEGER NOT NULL,
    paid INTEGER NOT NULL DEFAULT 0,
    unlocked INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (town_id, category, slot_id)
);

-- Unlockable state per room
CREATE TABLE unlockable_state (
    room_id INTEGER NOT NULL,
    unlockable_id INTEGER NOT NULL,
    available INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (room_id, unlockable_id)
);

-- Music changer state per room
CREATE TABLE music_changer_state (
    room_id INTEGER NOT NULL,
    slot_id INTEGER NOT NULL,
    day_unlocked INTEGER NOT NULL DEFAULT 0,
    night_unlocked INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (room_id, slot_id)
);

-- Warp center unlock state
CREATE TABLE warp_center_state (
    room_id INTEGER NOT NULL,
    slot_id INTEGER NOT NULL,
    warp_category INTEGER NOT NULL,
    unlocked INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (room_id, slot_id, warp_category)
);
```

## Example Configuration

```toml
# config/upgrader.toml

[[towns]]
town_id = 42  # New City

[[towns.upgrades]]
category = "Other"
slot = 1
name = "Add Bubblegum Machine"
need = 10000
unlocked = true
option = 1
other1 = 42   # room_id
other2 = 1    # unlockable_id

[[towns.upgrades]]
category = "Other"
slot = 4
name = "Unlock Music Changer"
need = 20000
unlocked = true
option = 1
other1 = 84
other2 = 1
# When completed, unlock slots 5 and 6 in Other category
unlock_chain = [
    { category = "Other", slot = 5 },
    { category = "Other", slot = 6 }
]

[[towns.upgrades]]
category = "Warp Center"
slot = 1
name = "Unlock Warp to Small-Cave"
need = 5000
unlocked = true
warp_room = 51
warp_slot = 1
warp_category = 2  # Fields

[[towns.upgrades]]
category = "Item Shop"
slot = 1
name = "Unlock new item"
need = 5000
unlocked = true
option = 1
other1 = 46   # shop room
other2 = 4    # slot to unlock
```

## Key Implementation Notes

1. **Community-wide system** - All players contribute to the same upgrades
2. **Points are pooled** - Multiple players' investments add up
3. **Upgrades are permanent** - Once unlocked, they remain unlocked forever
4. **Upgrade chains** - Some upgrades unlock other upgrade options when completed
5. **Per-town state** - Each city has its own upgrader and progress
6. **No RNG** - Investments always succeed and add to progress (no success rates)
7. **Real-time broadcasts** - When an upgrade completes, affected players are notified immediately
8. **Percentage display** - Client shows progress as percentage; 250 is special "locked" indicator
