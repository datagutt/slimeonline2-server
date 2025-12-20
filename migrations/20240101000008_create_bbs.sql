-- BBS (Bulletin Board System) posts table
CREATE TABLE IF NOT EXISTS bbs_posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    category_id INTEGER NOT NULL DEFAULT 0,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    is_reported INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for efficient category + pagination queries
CREATE INDEX IF NOT EXISTS idx_bbs_posts_category_created ON bbs_posts(category_id, created_at DESC);

-- Index for finding posts by character
CREATE INDEX IF NOT EXISTS idx_bbs_posts_character ON bbs_posts(character_id);

-- Table to track when users last posted (for rate limiting)
CREATE TABLE IF NOT EXISTS bbs_post_cooldowns (
    character_id INTEGER PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    last_post_at TEXT NOT NULL DEFAULT (datetime('now'))
);
