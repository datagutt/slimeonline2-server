-- Add hack tracking for persistent anti-cheat
-- GML behavior: MaxHacks = 8 triggers permanent ban

-- Add hack_count column to accounts table
ALTER TABLE accounts ADD COLUMN hack_count INTEGER NOT NULL DEFAULT 0;

-- Create hack_log table for detailed logging
CREATE TABLE IF NOT EXISTS hack_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL,
    character_id INTEGER,
    hack_type TEXT NOT NULL,
    description TEXT,
    ip_address TEXT,
    mac_address TEXT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (account_id) REFERENCES accounts(id)
);

-- Index for efficient lookups by account
CREATE INDEX IF NOT EXISTS idx_hack_log_account ON hack_log(account_id);
-- Index for time-based queries (recent hacks)
CREATE INDEX IF NOT EXISTS idx_hack_log_timestamp ON hack_log(timestamp);
