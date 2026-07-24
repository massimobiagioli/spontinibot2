CREATE TABLE IF NOT EXISTS training_message (
    id INTEGER PRIMARY KEY,
    session_id INTEGER NOT NULL REFERENCES training_session(id),
    question TEXT NOT NULL,
    answer TEXT NOT NULL,
    sources TEXT NOT NULL,
    fell_back INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
