-- Server state (key-value store for persistent server state like last restock date)
CREATE TABLE IF NOT EXISTS server_state (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
