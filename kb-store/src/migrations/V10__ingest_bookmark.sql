CREATE TABLE IF NOT EXISTS ingest_bookmark (
    id INTEGER PRIMARY KEY,
    section_id INTEGER NOT NULL REFERENCES ingest_section(id) ON DELETE CASCADE,
    source_url TEXT NOT NULL,
    last_item_ref TEXT NOT NULL,
    last_item_date TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(section_id, source_url)
);
