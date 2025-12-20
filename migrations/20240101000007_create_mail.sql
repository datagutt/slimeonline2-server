-- Mail system table
CREATE TABLE IF NOT EXISTS mail (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_character_id INTEGER NOT NULL REFERENCES characters(id),
    to_character_id INTEGER NOT NULL REFERENCES characters(id),
    sender_name TEXT NOT NULL,
    message TEXT NOT NULL,
    item_id INTEGER DEFAULT 0,
    points INTEGER DEFAULT 0,
    is_read INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT (datetime('now')),
    
    -- Index for faster mailbox queries
    FOREIGN KEY (from_character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (to_character_id) REFERENCES characters(id) ON DELETE CASCADE
);

-- Index for fetching player's mailbox efficiently
CREATE INDEX IF NOT EXISTS idx_mail_to_character ON mail(to_character_id, created_at DESC);

-- Index for counting unread mail
CREATE INDEX IF NOT EXISTS idx_mail_unread ON mail(to_character_id, is_read);
