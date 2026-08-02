CREATE TABLE IF NOT EXISTS robots_bypass_host (
    id INTEGER PRIMARY KEY,
    host TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Pre-populate with the one exception this repo's ingest system already
-- relied on before this table existed (the comune's own news site, whose
-- robots.txt disallows all non-search-engine crawlers) — see AGENTS.md's
-- "Scraper Exceptions Must Be Operator-Configured, Never Hard-Coded" rule.
INSERT OR IGNORE INTO robots_bypass_host (host) VALUES ('www.comune.maiolatispontini.an.it');
