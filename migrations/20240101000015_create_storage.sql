-- Extended storage for outfits, items, accessories, and tools
-- 20 pages x 9 slots = 180 slots per category per character

CREATE TABLE IF NOT EXISTS storage (
    character_id INTEGER NOT NULL,
    category INTEGER NOT NULL,  -- 1=outfits, 2=items, 3=accessories, 4=tools
    slot INTEGER NOT NULL,      -- 0-179 (page * 9 + slot_in_page)
    item_id INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (character_id, category, slot),
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_storage_char_cat ON storage(character_id, category);

-- One-time items tracking (items that can only be taken once per player)
CREATE TABLE IF NOT EXISTS one_time_items (
    room_id INTEGER NOT NULL,
    real_id INTEGER NOT NULL,   -- Item ID within the room
    category INTEGER NOT NULL,  -- 1=outfit, 2=item, 3=accessory
    item_id INTEGER NOT NULL,   -- Actual item/outfit/accessory ID
    PRIMARY KEY (room_id, real_id)
);

CREATE TABLE IF NOT EXISTS one_time_taken (
    character_id INTEGER NOT NULL,
    room_id INTEGER NOT NULL,
    real_id INTEGER NOT NULL,
    taken_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (character_id, room_id, real_id),
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE
);

-- Race records and leaderboards
CREATE TABLE IF NOT EXISTS race_records (
    race_id INTEGER NOT NULL,
    record_type TEXT NOT NULL,  -- 'single' or 'clan'
    rank INTEGER NOT NULL,      -- 1-10
    name TEXT NOT NULL,
    time_ms INTEGER NOT NULL,
    character_id INTEGER,       -- NULL for clan records
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (race_id, record_type, rank)
);

CREATE TABLE IF NOT EXISTS race_config (
    race_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    time_limit_ms INTEGER NOT NULL DEFAULT 0,
    reward_category INTEGER,
    reward_item_id INTEGER,
    reward_points INTEGER DEFAULT 0
);

-- Insert default race config
INSERT OR IGNORE INTO race_config (race_id, name, time_limit_ms) VALUES (1, 'Magma Dungeon', 60000);

-- Music changer state per room
CREATE TABLE IF NOT EXISTS music_changer_state (
    room_id INTEGER PRIMARY KEY,
    current_day_music INTEGER NOT NULL DEFAULT 0,
    current_night_music INTEGER NOT NULL DEFAULT 0,
    day_track_1 INTEGER NOT NULL DEFAULT 1,
    day_track_2 INTEGER NOT NULL DEFAULT 2,
    day_track_3 INTEGER NOT NULL DEFAULT 3,
    day_track_3_unlocked INTEGER NOT NULL DEFAULT 0,
    night_track_1 INTEGER NOT NULL DEFAULT 4,
    night_track_2 INTEGER NOT NULL DEFAULT 5,
    night_track_3 INTEGER NOT NULL DEFAULT 6,
    night_track_3_unlocked INTEGER NOT NULL DEFAULT 0,
    cooldown_until TEXT  -- ISO8601 datetime when cooldown ends
);
