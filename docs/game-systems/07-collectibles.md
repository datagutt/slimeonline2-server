# Collectible System

**See:** [`../protocol/04-message-catalog.md`](../protocol/04-message-catalog.md) - Collectibles section

## Collectible Messages

- MSG_COLLECTIBLE_INFO (32) - Send collectible info when entering room
- MSG_COLLECTIBLE_TAKE_SELF (33) - Player picked up collectible (to self)
- MSG_COLLECTIBLE_TAKEN (34) - Collectible was taken (broadcast to room)
- MSG_COLLECTIBLE_EVOLVE (132) - Collectible evolved to new item

## How Collectibles Work

Collectibles are items that spawn at fixed locations in rooms. When a player picks one up:
1. Server validates the collectible exists and is available
2. Server gives item to player's inventory
3. Server marks collectible as unavailable
4. Server broadcasts MSG_COLLECTIBLE_TAKEN to room
5. Server starts respawn timer
6. After respawn time, collectible becomes available again

## Original Server File Format

Collectibles are defined in room `.default` files:

```ini
[Collectibles]
1 id=22              # Current item ID (may change if evolving)
1 original id=22     # Original item ID (for reset after evolution)
1 avail=1            # 1=available, 0=taken
1 x=196              # X position
1 y=56               # Y position
1 respawn=20         # Base respawn time in minutes
1 varspawn=20        # Random variance in minutes (0 to varspawn added)
1 cur-spawn=0        # Current countdown (runtime state)
```

**Respawn Formula:**
```
actual_respawn = respawn + random(0, varspawn)
```

## Collectible Items in Original Server

The most common collectible items:
- **22** (Magmanis) - Found in volcanic/lava areas
- **57** (Blazing Bubble) - Found in volcanic areas
- **60** (Bell Twig) - Found in forest areas

## Example Collectible Spawn Points

### Room 100 (Lava Area)
| # | Item | Position | Respawn | Variance |
|---|------|----------|---------|----------|
| 1 | 22 (Magmanis) | 196, 56 | 20 min | +0-20 min |
| 2 | 22 (Magmanis) | 112, 376 | 30 min | +0-20 min |
| 3 | 57 (Blazing Bubble) | 176, 64 | 40 min | +0-30 min |

### Room 101 (Lava Area)
| # | Item | Position | Respawn | Variance |
|---|------|----------|---------|----------|
| 1 | 22 (Magmanis) | 488, 216 | 30 min | +0-15 min |
| 2 | 22 (Magmanis) | 520, 216 | 30 min | +0-10 min |
| 3 | 57 (Blazing Bubble) | 416, 264 | 50 min | +0-50 min |
| 4 | 57 (Blazing Bubble) | 192, 200 | 60 min | +0-20 min |

### Room 109 (Volcanic)
| # | Item | Position | Respawn | Variance |
|---|------|----------|---------|----------|
| 1 | 57 (Blazing Bubble) | 192, 568 | 60 min | +0-60 min |
| 2 | 57 (Blazing Bubble) | 296, 288 | 60 min | +0-30 min |
| 3 | 57 (Blazing Bubble) | 864, 552 | 40 min | +0-60 min |
| 4 | 22 (Magmanis) | 376, 280 | 30 min | +0-10 min |
| 5 | 57 (Blazing Bubble) | 896, 328 | 40 min | +0-80 min |
| 6 | 57 (Blazing Bubble) | 856, 136 | 80 min | +0-80 min |

### Forest Rooms (115-120)
| Room | Item | Position | Respawn | Variance |
|------|------|----------|---------|----------|
| 115 | 60 (Bell Twig) | 704, 392 | 30 min | +0-60 min |
| 116 | 60 (Bell Twig) | 496, 184 | 20 min | +0-40 min |
| 117 | 60 (Bell Twig) | 600, 616 | 80 min | +0-80 min |
| 119 | 60 (Bell Twig) | 336, 104 | 80 min | +0-100 min |
| 120 | 60 (Bell Twig) | 248, 472 | 30 min | +0-80 min |

## Evolving Collectibles

Some collectibles can "evolve" into different items over time. This is configured in `evolvingCollectibles.data`:

```ini
[20]
Next=58           # Evolves into item 58
Minutes=60        # After 60 minutes
VarMinutes=20     # +0-20 variance

[58]
Next=59           # Item 58 evolves into 59
Minutes=10
VarMinutes=20
```

**Evolution Chain Example:**
- Item 20 (Red Mushroom) → Item 58 (Squishy Mushroom) after ~60-80 min
- Item 58 (Squishy Mushroom) → Item 59 (Stinky Mushroom) after ~10-30 min

## Rust Server Implementation

Our Rust server uses **config files as the single source of truth** for spawn definitions. The database only stores runtime state (availability, respawn timers).

### collectibles.toml
```toml
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
]

# Evolving collectibles
[evolving]
20 = { to = 58, minutes = 60, variance = 20 }  # Red Mushroom -> Squishy
58 = { to = 59, minutes = 10, variance = 20 }  # Squishy -> Stinky
```

### Database (runtime state only)
```sql
CREATE TABLE collectible_state (
    room_id INTEGER NOT NULL,
    spawn_id INTEGER NOT NULL,
    available INTEGER DEFAULT 1,
    respawn_at TEXT,
    PRIMARY KEY (room_id, spawn_id)
);
```

Or via configuration file:
```toml
# collectibles.toml
[[room.100]]
index = 1
item_id = 22
x = 196
y = 56
respawn = 20
variance = 20

[[evolving]]
from = 20
to = 58
minutes = 60
variance = 20
```

## Validation

- Verify room has collectibles section
- Verify collectible index exists
- Verify collectible is currently available
- Verify player has free item inventory slot
- Reset item_id to original_id after taking (for evolving collectibles)

See [`../security/02-server-validation.md`](../security/02-server-validation.md).
