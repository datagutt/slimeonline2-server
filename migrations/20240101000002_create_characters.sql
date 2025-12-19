-- Create characters table
CREATE TABLE IF NOT EXISTS characters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL UNIQUE REFERENCES accounts(id) ON DELETE CASCADE,
    username TEXT NOT NULL,
    
    -- Position
    -- Note: room_id 37 = rm_around_new_1 (main spawn area)
    x INTEGER NOT NULL DEFAULT 160,
    y INTEGER NOT NULL DEFAULT 120,
    room_id INTEGER NOT NULL DEFAULT 37,
    
    -- Appearance
    body_id INTEGER NOT NULL DEFAULT 1,
    acs1_id INTEGER NOT NULL DEFAULT 0,
    acs2_id INTEGER NOT NULL DEFAULT 0,
    
    -- Currency & Stats
    points INTEGER NOT NULL DEFAULT 0 CHECK (points >= 0),
    bank_balance INTEGER NOT NULL DEFAULT 0 CHECK (bank_balance >= 0),
    trees_planted INTEGER NOT NULL DEFAULT 0,
    objects_built INTEGER NOT NULL DEFAULT 0,
    
    -- Quest State
    quest_id INTEGER NOT NULL DEFAULT 0,
    quest_step INTEGER NOT NULL DEFAULT 0,
    quest_var INTEGER NOT NULL DEFAULT 0,
    
    -- Permissions
    has_signature INTEGER DEFAULT 0,
    is_moderator INTEGER DEFAULT 0,
    
    -- Clan
    clan_id INTEGER REFERENCES clans(id) ON DELETE SET NULL,
    
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_characters_account ON characters(account_id);
CREATE INDEX IF NOT EXISTS idx_characters_room ON characters(room_id);
