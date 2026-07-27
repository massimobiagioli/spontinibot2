ALTER TABLE documents ADD COLUMN section TEXT;

CREATE INDEX IF NOT EXISTS idx_documents_section ON documents(section);
