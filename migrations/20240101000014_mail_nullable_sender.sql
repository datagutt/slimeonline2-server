-- Make from_character_id nullable for system mail
-- SQLite doesn't support ALTER COLUMN, so we need to recreate the table

-- Create new table with nullable from_character_id
CREATE TABLE IF NOT EXISTS mail_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_character_id INTEGER REFERENCES characters(id) ON DELETE CASCADE,
    to_character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    sender_name TEXT NOT NULL,
    message TEXT NOT NULL,
    item_id INTEGER DEFAULT 0,
    item_cat INTEGER DEFAULT 0,
    points INTEGER DEFAULT 0,
    paper INTEGER DEFAULT 1,
    font_color INTEGER DEFAULT 0,
    is_read INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT (datetime('now'))
);

-- Copy data from old table
INSERT INTO mail_new (id, from_character_id, to_character_id, sender_name, message, item_id, item_cat, points, paper, font_color, is_read, created_at)
SELECT id, from_character_id, to_character_id, sender_name, message, item_id, item_cat, points, paper, font_color, is_read, created_at
FROM mail;

-- Drop old table
DROP TABLE mail;

-- Rename new table
ALTER TABLE mail_new RENAME TO mail;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_mail_to_character ON mail(to_character_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_mail_unread ON mail(to_character_id, is_read);
