CREATE TABLE IF NOT EXISTS training_feedback (
    id INTEGER PRIMARY KEY,
    message_id INTEGER NOT NULL REFERENCES training_message(id),
    chunk_id INTEGER REFERENCES documents(id),
    answer_span TEXT NOT NULL,
    sentiment TEXT NOT NULL,
    comment TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
