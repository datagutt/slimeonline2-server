-- Create bans table
CREATE TABLE IF NOT EXISTS bans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ban_type TEXT NOT NULL, -- 'ip', 'account', 'mac'
    value TEXT NOT NULL,
    reason TEXT NOT NULL,
    banned_by TEXT,
    banned_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT, -- NULL = permanent
    
    UNIQUE(ban_type, value)
);

CREATE INDEX IF NOT EXISTS idx_bans_type_value ON bans(ban_type, value);
