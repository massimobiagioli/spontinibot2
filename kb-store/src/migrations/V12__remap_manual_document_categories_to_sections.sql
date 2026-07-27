-- V11 copied `metadata.category` verbatim into `documents.section` for
-- manual uploads, but the historical category taxonomy used at ingestion
-- time ('delibera', 'determina', 'civic', 'roster') does not match the real
-- `ingest_section.name` values ('storia', 'news', 'delibere', 'giunta').
-- Remap the recovered values to the real section names so the admin-ui's
-- per-section document list actually finds them.
--
-- 'orari' (a single document whose content is verbatim the BDD e2e test
-- fixture text, not real municipal content — test data that leaked into
-- the shared kb.db during an end-to-end test run) is deliberately left
-- unmapped here; remapping it to a real section would misrepresent it as
-- genuine content. Cleaning up that stray test row is a separate concern.
UPDATE documents SET section = 'delibere' WHERE section IN ('delibera', 'determina');
UPDATE documents SET section = 'news' WHERE section = 'civic';
UPDATE documents SET section = 'giunta' WHERE section = 'roster';
