-- V9 recovered `documents.section` for scraped rows from `metadata.section`
-- (the chunker's own field), and explicitly left manual uploads' NULL
-- section alone on the belief that information was never captured. It was:
-- every manually-uploaded document's `metadata.category` already records
-- the same section it was uploaded into (see the "auto-derive upload
-- metadata" feature, which sets `category = Some(section.clone())`).
-- Recover it here for the one source type V9 did not cover.
UPDATE documents
SET section = json_extract(metadata, '$.category')
WHERE section IS NULL
  AND source = 'manual'
  AND metadata IS NOT NULL
  AND json_valid(metadata)
  AND json_extract(metadata, '$.category') IS NOT NULL;
