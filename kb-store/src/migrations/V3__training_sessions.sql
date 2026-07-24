CREATE TABLE IF NOT EXISTS training_session (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_by TEXT,
    closed_at TEXT
);
