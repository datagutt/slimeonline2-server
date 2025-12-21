-- Runtime state tables for collectibles, plants, and shop stock
-- These track the current state of world objects that persist across server restarts

-- Collectible runtime state (tracks which collectibles are currently available)
-- Respawn times are calculated from config, this just tracks availability
CREATE TABLE IF NOT EXISTS collectible_state (
    room_id INTEGER NOT NULL,
    spawn_id INTEGER NOT NULL,
    available INTEGER NOT NULL DEFAULT 1,  -- 1 = available, 0 = taken
    respawn_at TEXT,                       -- ISO8601 datetime when it respawns (NULL if available)
    current_item_id INTEGER,               -- Current item ID (may differ from config if evolved)
    PRIMARY KEY (room_id, spawn_id)
);

-- Index for finding collectibles that need to respawn
CREATE INDEX IF NOT EXISTS idx_collectible_respawn ON collectible_state(respawn_at) WHERE available = 0;

-- Plant runtime state (tracks planted trees and their growth)
CREATE TABLE IF NOT EXISTS plant_state (
    room_id INTEGER NOT NULL,
    spot_id INTEGER NOT NULL,
    owner_id INTEGER,                      -- character_id of owner (NULL if empty)
    seed_id INTEGER,                       -- seed item ID that was planted
    stage INTEGER NOT NULL DEFAULT 0,      -- growth stage (0-5: growing, 6: has fruit, 7: dead)
    fairy_count INTEGER NOT NULL DEFAULT 0, -- number of fairies added (0-5)
    pinwheel_id INTEGER,                   -- pinwheel item ID if added (NULL if none)
    planted_at TEXT,                       -- ISO8601 datetime when planted
    next_stage_at TEXT,                    -- ISO8601 datetime for next growth stage
    has_fruit INTEGER NOT NULL DEFAULT 0,  -- 1 if fruit is ready to harvest
    PRIMARY KEY (room_id, spot_id),
    FOREIGN KEY (owner_id) REFERENCES characters(id) ON DELETE SET NULL
);

-- Index for finding plants that need to advance growth stage
CREATE INDEX IF NOT EXISTS idx_plant_growth ON plant_state(next_stage_at) WHERE stage < 6;

-- Shop stock runtime state (tracks current stock for limited items)
-- Only stores entries for items that have been purchased (stock < max)
CREATE TABLE IF NOT EXISTS shop_stock (
    room_id INTEGER NOT NULL,
    slot_id INTEGER NOT NULL,
    current_stock INTEGER NOT NULL,        -- remaining stock
    last_purchase TEXT,                    -- ISO8601 datetime of last purchase
    last_restock TEXT,                     -- ISO8601 datetime of last restock
    PRIMARY KEY (room_id, slot_id)
);

-- Discarded items on the ground (dropped by players)
-- These persist until picked up or server restart (configurable)
CREATE TABLE IF NOT EXISTS ground_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,
    item_id INTEGER NOT NULL,
    x INTEGER NOT NULL,
    y INTEGER NOT NULL,
    dropped_by INTEGER,                    -- character_id who dropped it (NULL if system)
    dropped_at TEXT NOT NULL,              -- ISO8601 datetime when dropped
    expires_at TEXT,                       -- ISO8601 datetime when it despawns (NULL = never)
    FOREIGN KEY (dropped_by) REFERENCES characters(id) ON DELETE SET NULL
);

-- Index for finding ground items by room
CREATE INDEX IF NOT EXISTS idx_ground_items_room ON ground_items(room_id);
-- Index for finding expired ground items
CREATE INDEX IF NOT EXISTS idx_ground_items_expire ON ground_items(expires_at) WHERE expires_at IS NOT NULL;
