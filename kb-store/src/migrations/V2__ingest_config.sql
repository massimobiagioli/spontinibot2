CREATE TABLE IF NOT EXISTS ingest_schedule (
    id INTEGER PRIMARY KEY DEFAULT 1,
    cron_expr TEXT NOT NULL DEFAULT '0 */6 * * *',
    enabled BOOLEAN NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS ingest_section (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    ordering INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS ingest_source (
    id INTEGER PRIMARY KEY,
    section_id INTEGER NOT NULL REFERENCES ingest_section(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL CHECK(source_type IN ('scrape','api')),
    url TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_source_section ON ingest_source(section_id);

CREATE TABLE IF NOT EXISTS ingest_run_request (
    id INTEGER PRIMARY KEY,
    requested_at TEXT NOT NULL DEFAULT (datetime('now')),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','running','done','failed'))
);
