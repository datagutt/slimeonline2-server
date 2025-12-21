# Planting System

**See:** [`../protocol/04-message-catalog.md`](../protocol/04-message-catalog.md) - Planting section

## Plant Messages

- MSG_PLANT_SPOT_FREE (63) - Plant spot is available
- MSG_PLANT_SPOT_USED (64) - Plant spot is in use
- MSG_PLANT_DIE (65) - Plant died/disappeared
- MSG_PLANT_GROW (66) - Plant grew to next stage
- MSG_PLANT_ADD_PINWHEEL (67) - Pinwheel added to plant
- MSG_PLANT_ADD_FAIRY (68) - Fairy added to plant
- MSG_PLANT_GET_FRUIT (69) - Player takes fruit
- MSG_PLANT_HAS_FRUIT (70) - Plant has fruit available
- MSG_TREE_PLANTED_INC (94) - Increment player's trees_planted stat
- MSG_PLANT_SET - Client wants to plant a seed
- MSG_PLANT_TAKE_FRUIT - Client takes fruit from tree

## Plantable Seeds

| Item ID | Name | Notes |
|---------|------|-------|
| 9 | Simple Seed | Basic tree |
| 24 | Blue Seed | Special tree variant |

## Growth Stages

Trees go through 6 stages:

| Stage | Description | Duration (from plant.rates) |
|-------|-------------|-----------------------------|
| 0 | Just planted | → Stage 1 |
| 1 | Small sprout | to1 minutes |
| 2 | Growing | to2 minutes |
| 3 | Nearly done | to3 minutes |
| 4 | Full grown | to4 minutes |
| 5 | Has fruit | to5 minutes (fruit available) |
| 6 | Dying | to6 minutes until disappear |

## Original Server Configuration

### plant.rates File

```ini
[Sample]
to1=How many minutes till it gets from "planted stick" to next step
to2=How many minutes till it gets from "lil grown" to next step
to3=How many minutes till it gets from "nearly done" to next step
to4=How many minutes till it gets from "done" to "done and fruits nao"
to5=How many minutes the tree stays "done and fruits"
to6=How many minutes it takes until the tree disappears
fruit1-5=What items can grow as fruit (equal chance)
chance=Base % chance for fruit (0-100, in 10-steps)

[9 Rates]
to1=240
to2=240
to3=360
to4=360
to5=720
to6=60
fruit1=9
fruit2=9
fruit3=9
fruit4=9
fruit5=9
chance=50

[24 Rates]
to1=240
to2=360
to3=360
to4=480
to5=720
to6=60
fruit1=25
fruit2=25
fruit3=24
fruit4=25
fruit5=25
chance=35
```

### Simple Seed (ID 9) Growth Times

| Stage | Duration | Total Time |
|-------|----------|------------|
| to1 | 240 min (4h) | 4h |
| to2 | 240 min (4h) | 8h |
| to3 | 360 min (6h) | 14h |
| to4 | 360 min (6h) | 20h |
| to5 | 720 min (12h) | 32h (fruit available) |
| to6 | 60 min (1h) | 33h (disappears) |

**Fruit:** Always drops Simple Seed (ID 9)
**Base Chance:** 50%

### Blue Seed (ID 24) Growth Times

| Stage | Duration | Total Time |
|-------|----------|------------|
| to1 | 240 min (4h) | 4h |
| to2 | 360 min (6h) | 10h |
| to3 | 360 min (6h) | 16h |
| to4 | 480 min (8h) | 24h |
| to5 | 720 min (12h) | 36h (fruit available) |
| to6 | 60 min (1h) | 37h (disappears) |

**Fruit:** Juicy Bango (25), Blue Seed (24)
**Base Chance:** 35%

## Fairies and Pinwheels

Players can add items to planted trees:

| Item | Effect |
|------|--------|
| Fairy (10) | +10% fruit chance (max 5 per tree = +50%) |
| Blue Pinwheel (11) | Decoration |
| Red Pinwheel (12) | Decoration |
| Glow Pinwheel (13) | Decoration |

**Max Fairies:** 5 per tree

## Plant Spot File Format

In room `.default` files:

```ini
[Plant Spots]
1free=1          # 1=available, 0=in use
1plant=0         # Seed item ID (0=empty)
1step=0          # Current growth stage
1fruit1=0        # Fruit in slot 1
1fruit2=0        # Fruit in slot 2
1fruit3=0        # Fruit in slot 3
1wheel=0         # Pinwheel type
1fairies=0       # Number of fairies
1time=0          # Time until next stage (runtime)
1owner=0         # Player ID who planted
```

## Rooms with Plant Spots

From the original server data:
- Room 104, 105, 106 (1 spot each)
- Room 111 (2 spots)
- Other rooms may have spots

## Rust Server Implementation

```sql
CREATE TABLE plant_spots (
    id SERIAL PRIMARY KEY,
    room_id SMALLINT NOT NULL,
    spot_index SMALLINT NOT NULL,
    owner_id INTEGER REFERENCES characters(id),
    seed_id SMALLINT DEFAULT 0,
    growth_stage SMALLINT DEFAULT 0,
    pinwheel_type SMALLINT DEFAULT 0,
    fairy_count SMALLINT DEFAULT 0,
    fruit_1 SMALLINT DEFAULT 0,
    fruit_2 SMALLINT DEFAULT 0,
    fruit_3 SMALLINT DEFAULT 0,
    next_stage_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    UNIQUE(room_id, spot_index)
);

CREATE TABLE plant_config (
    seed_id SMALLINT PRIMARY KEY,
    stage_1_minutes SMALLINT NOT NULL,
    stage_2_minutes SMALLINT NOT NULL,
    stage_3_minutes SMALLINT NOT NULL,
    stage_4_minutes SMALLINT NOT NULL,
    fruit_duration_minutes SMALLINT NOT NULL,
    death_duration_minutes SMALLINT NOT NULL,
    fruit_chance SMALLINT NOT NULL,
    possible_fruits SMALLINT[] NOT NULL
);
```

Configuration file alternative:
```toml
# plants.toml
[seed.9]
name = "Simple Seed"
stages = [240, 240, 360, 360, 720, 60]
fruits = [9, 9, 9, 9, 9]
chance = 50

[seed.24]
name = "Blue Seed"
stages = [240, 360, 360, 480, 720, 60]
fruits = [25, 25, 24, 25, 25]
chance = 35
```

## Validation

- Verify player has seed in inventory
- Verify room has plant spots
- Verify plant spot is free
- Verify player owns tree when taking fruit
- Fairy count max 5

See [`../security/02-server-validation.md`](../security/02-server-validation.md).
