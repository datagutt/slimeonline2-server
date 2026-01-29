-- Add room_id to BBS posts for per-city bulletin boards
ALTER TABLE bbs_posts ADD COLUMN room_id INTEGER NOT NULL DEFAULT 0;

-- Update index for efficient room + category + pagination queries
DROP INDEX IF EXISTS idx_bbs_posts_category_created;
CREATE INDEX IF NOT EXISTS idx_bbs_posts_room_category_created ON bbs_posts(room_id, category_id, created_at DESC);
