-- Upgrader system tables for community investment upgrades

-- Track investment progress per upgrade slot
CREATE TABLE IF NOT EXISTS upgrader_state (
    town_id INTEGER NOT NULL,
    category TEXT NOT NULL,
    slot_id INTEGER NOT NULL,
    paid INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (town_id, category, slot_id)
);

-- Track which upgrade slots have been unlocked (made visible)
-- This is separate from completion - slots must be unlocked before players can see/invest in them
CREATE TABLE IF NOT EXISTS upgrader_unlocked (
    town_id INTEGER NOT NULL,
    category TEXT NOT NULL,
    slot_id INTEGER NOT NULL,
    unlocked INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (town_id, category, slot_id)
);

-- Track unlockable objects in rooms (bubblegum machines, decorations, etc.)
CREATE TABLE IF NOT EXISTS unlockable_state (
    room_id INTEGER NOT NULL,
    unlockable_id INTEGER NOT NULL,
    available INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (room_id, unlockable_id)
);

-- Track music changer song unlocks
CREATE TABLE IF NOT EXISTS music_changer_state (
    room_id INTEGER NOT NULL,
    slot_id INTEGER NOT NULL,
    day_unlocked INTEGER NOT NULL DEFAULT 0,
    night_unlocked INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (room_id, slot_id)
);

-- Track warp center destination unlocks
CREATE TABLE IF NOT EXISTS warp_center_state (
    room_id INTEGER NOT NULL,
    slot_id INTEGER NOT NULL,
    warp_category INTEGER NOT NULL,
    unlocked INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (room_id, slot_id, warp_category)
);

-- Track shop slot unlocks (for upgrades that add new items to shops)
CREATE TABLE IF NOT EXISTS shop_slot_unlocked (
    room_id INTEGER NOT NULL,
    slot_id INTEGER NOT NULL,
    available INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (room_id, slot_id)
);

-- Track permanent stock bonuses per room (from upgrader investments)
-- This bonus is added to the max_stock from config for all items in the room
CREATE TABLE IF NOT EXISTS shop_stock_bonus (
    room_id INTEGER NOT NULL PRIMARY KEY,
    bonus INTEGER NOT NULL DEFAULT 0
);
