CREATE TABLE IF NOT EXISTS documents (
    id INTEGER PRIMARY KEY,
    source TEXT,
    source_ref TEXT,
    content TEXT,
    metadata TEXT,
    embedding F32_BLOB(768)
);

CREATE TABLE IF NOT EXISTS persona (
    id INTEGER PRIMARY KEY,
    version INTEGER NOT NULL,
    name TEXT NOT NULL,
    system_prompt TEXT NOT NULL,
    tone TEXT,
    fallback_message TEXT,
    is_active BOOLEAN DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    created_by TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_persona_active ON persona(is_active) WHERE is_active = 1;
