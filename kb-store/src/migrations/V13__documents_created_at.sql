-- The `documents` table never had a `created_at` column — the admin-ui's
-- per-section ingested-content list had no way to sort recently-added
-- content to the top, or show an operator when a document was ingested.
--
-- No DEFAULT clause here: libsql rejects a non-constant default (even the
-- `CURRENT_TIMESTAMP` keyword) on `ALTER TABLE ... ADD COLUMN`, unlike plain
-- SQLite. `insert_document`'s own INSERT statement sets `created_at` to
-- `datetime('now')` explicitly instead, so new rows still get a real
-- timestamp automatically.
--
-- Pre-existing rows have no real historical ingestion timestamp to recover
-- (it was never recorded) — backfill them to the time this migration ran,
-- an honest fact, not a fabricated ingestion date.
ALTER TABLE documents ADD COLUMN created_at TEXT;

UPDATE documents SET created_at = datetime('now') WHERE created_at IS NULL;
