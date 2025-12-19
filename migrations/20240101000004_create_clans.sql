-- Create clans table
CREATE TABLE IF NOT EXISTS clans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    leader_id INTEGER NOT NULL REFERENCES characters(id),
    
    -- Display
    color_inner INTEGER NOT NULL DEFAULT 0,
    color_outer INTEGER NOT NULL DEFAULT 0,
    
    -- Stats
    level INTEGER NOT NULL DEFAULT 1,
    points INTEGER NOT NULL DEFAULT 0,
    max_members INTEGER NOT NULL DEFAULT 5,
    
    -- Info
    description TEXT,
    news TEXT,
    show_name INTEGER DEFAULT 1,
    has_base INTEGER DEFAULT 0,
    
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_clans_leader ON clans(leader_id);
