-- Building state table for tracking placed buildings
-- Buildings are temporary structures placed by players that expire after a set duration

CREATE TABLE IF NOT EXISTS building_state (
    room_id INTEGER NOT NULL,
    spot_id INTEGER NOT NULL,
    owner_id INTEGER,                      -- character_id of who built it (NULL if empty)
    object_id INTEGER,                     -- item ID of the building object (e.g., 26 = Weak Cannon)
    built_at TEXT,                         -- ISO8601 datetime when built
    expires_at TEXT,                       -- ISO8601 datetime when building expires
    PRIMARY KEY (room_id, spot_id),
    FOREIGN KEY (owner_id) REFERENCES characters(id) ON DELETE SET NULL
);

-- Index for finding buildings that need to expire
CREATE INDEX IF NOT EXISTS idx_building_expires ON building_state(expires_at) WHERE owner_id IS NOT NULL;
