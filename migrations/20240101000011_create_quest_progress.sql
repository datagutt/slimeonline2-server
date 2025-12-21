-- Create quest progress table to track cleared quests
CREATE TABLE IF NOT EXISTS quest_progress (
    character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    quest_id INTEGER NOT NULL,
    cleared INTEGER NOT NULL DEFAULT 0,
    cleared_at TEXT,
    PRIMARY KEY (character_id, quest_id)
);

CREATE INDEX IF NOT EXISTS idx_quest_progress_character ON quest_progress(character_id);
