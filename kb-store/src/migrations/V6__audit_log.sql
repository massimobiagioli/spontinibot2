CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    target TEXT NOT NULL,
    payload TEXT NOT NULL,
    at TEXT NOT NULL DEFAULT (datetime('now'))
);
