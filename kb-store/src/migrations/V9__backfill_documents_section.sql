-- Documents scraped or manually chunked before V8 added `documents.section`
-- still carry the section name inside their `metadata` JSON (set by
-- ingest-core's chunker as `{"section": ..., "source_url": ...}`) — recover
-- it for any row where the column itself was never populated. Rows whose
-- metadata never recorded a section (e.g. manual uploads made before this
-- change, which only stored category/tags/trust_score) are left untouched;
-- that information was never captured and cannot be recovered.
UPDATE documents
SET section = json_extract(metadata, '$.section')
WHERE section IS NULL
  AND metadata IS NOT NULL
  AND json_valid(metadata)
  AND json_extract(metadata, '$.section') IS NOT NULL;
