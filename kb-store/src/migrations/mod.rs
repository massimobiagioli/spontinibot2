use crate::error::Result;
use libsql::Connection;

const V1_SCHEMA: &str = include_str!("V1__initial_schema.sql");
const V2_SCHEMA: &str = include_str!("V2__ingest_config.sql");
const V3_SCHEMA: &str = include_str!("V3__training_sessions.sql");
const V4_SCHEMA: &str = include_str!("V4__training_messages.sql");
const V5_SCHEMA: &str = include_str!("V5__training_feedback.sql");
const V6_SCHEMA: &str = include_str!("V6__audit_log.sql");
const V7_SCHEMA: &str = include_str!("V7__training_redesign.sql");
const V8_SCHEMA: &str = include_str!("V8__documents_section.sql");
const V9_SCHEMA: &str = include_str!("V9__backfill_documents_section.sql");
const V10_SCHEMA: &str = include_str!("V10__ingest_bookmark.sql");
const V11_SCHEMA: &str = include_str!("V11__backfill_manual_documents_section.sql");
const V12_SCHEMA: &str = include_str!("V12__remap_manual_document_categories_to_sections.sql");
const V13_SCHEMA: &str = include_str!("V13__documents_created_at.sql");
const V14_SCHEMA: &str = include_str!("V14__robots_bypass_hosts.sql");

/// Run database migrations idempotently.
///
/// Creates the `_migrations` tracking table if it does not exist,
/// applies the base schema (idempotent via `IF NOT EXISTS`),
/// and records the migration as applied.
pub async fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .await?;

    let mut rows = conn
        .query(
            "SELECT 1 FROM _migrations WHERE version = 1",
            libsql::params![],
        )
        .await?;
    if rows.next().await?.is_none() {
        let tx = conn.transaction().await?;
        tx.execute_batch(V1_SCHEMA).await?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (1, 'initial_schema')",
            libsql::params![],
        )
        .await?;
        tx.commit().await?;
    }

    let mut rows = conn
        .query(
            "SELECT 1 FROM _migrations WHERE version = 2",
            libsql::params![],
        )
        .await?;
    if rows.next().await?.is_none() {
        let tx = conn.transaction().await?;
        tx.execute_batch(V2_SCHEMA).await?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (2, 'ingest_config_schema')",
            libsql::params![],
        )
        .await?;
        tx.commit().await?;
    }

    let mut rows = conn
        .query(
            "SELECT 1 FROM _migrations WHERE version = 3",
            libsql::params![],
        )
        .await?;
    if rows.next().await?.is_none() {
        let tx = conn.transaction().await?;
        tx.execute_batch(V3_SCHEMA).await?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (3, 'training_sessions_schema')",
            libsql::params![],
        )
        .await?;
        tx.commit().await?;
    }

    let mut rows = conn
        .query(
            "SELECT 1 FROM _migrations WHERE version = 4",
            libsql::params![],
        )
        .await?;
    if rows.next().await?.is_none() {
        let tx = conn.transaction().await?;
        tx.execute_batch(V4_SCHEMA).await?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (4, 'training_messages_schema')",
            libsql::params![],
        )
        .await?;
        tx.commit().await?;
    }

    let mut rows = conn
        .query(
            "SELECT 1 FROM _migrations WHERE version = 5",
            libsql::params![],
        )
        .await?;
    if rows.next().await?.is_none() {
        let tx = conn.transaction().await?;
        tx.execute_batch(V5_SCHEMA).await?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (5, 'training_feedback_schema')",
            libsql::params![],
        )
        .await?;
        tx.commit().await?;
    }

    let mut rows = conn
        .query(
            "SELECT 1 FROM _migrations WHERE version = 6",
            libsql::params![],
        )
        .await?;
    if rows.next().await?.is_none() {
        let tx = conn.transaction().await?;
        tx.execute_batch(V6_SCHEMA).await?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (6, 'audit_log_schema')",
            libsql::params![],
        )
        .await?;
        tx.commit().await?;
    }

    let mut rows = conn
        .query(
            "SELECT 1 FROM _migrations WHERE version = 7",
            libsql::params![],
        )
        .await?;
    if rows.next().await?.is_none() {
        let tx = conn.transaction().await?;
        tx.execute_batch(V7_SCHEMA).await?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (7, 'training_redesign_schema')",
            libsql::params![],
        )
        .await?;
        tx.commit().await?;
    }

    let mut rows = conn
        .query(
            "SELECT 1 FROM _migrations WHERE version = 8",
            libsql::params![],
        )
        .await?;
    if rows.next().await?.is_none() {
        let tx = conn.transaction().await?;
        tx.execute_batch(V8_SCHEMA).await?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (8, 'documents_section_schema')",
            libsql::params![],
        )
        .await?;
        tx.commit().await?;
    }

    let mut rows = conn
        .query(
            "SELECT 1 FROM _migrations WHERE version = 9",
            libsql::params![],
        )
        .await?;
    if rows.next().await?.is_none() {
        let tx = conn.transaction().await?;
        tx.execute_batch(V9_SCHEMA).await?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (9, 'backfill_documents_section')",
            libsql::params![],
        )
        .await?;
        tx.commit().await?;
    }

    let mut rows = conn
        .query(
            "SELECT 1 FROM _migrations WHERE version = 10",
            libsql::params![],
        )
        .await?;
    if rows.next().await?.is_none() {
        let tx = conn.transaction().await?;
        tx.execute_batch(V10_SCHEMA).await?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (10, 'ingest_bookmark_schema')",
            libsql::params![],
        )
        .await?;
        tx.commit().await?;
    }

    let mut rows = conn
        .query(
            "SELECT 1 FROM _migrations WHERE version = 11",
            libsql::params![],
        )
        .await?;
    if rows.next().await?.is_none() {
        let tx = conn.transaction().await?;
        tx.execute_batch(V11_SCHEMA).await?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (11, 'backfill_manual_documents_section')",
            libsql::params![],
        )
        .await?;
        tx.commit().await?;
    }

    let mut rows = conn
        .query(
            "SELECT 1 FROM _migrations WHERE version = 12",
            libsql::params![],
        )
        .await?;
    if rows.next().await?.is_none() {
        let tx = conn.transaction().await?;
        tx.execute_batch(V12_SCHEMA).await?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (12, 'remap_manual_document_categories_to_sections')",
            libsql::params![],
        )
        .await?;
        tx.commit().await?;
    }

    let mut rows = conn
        .query(
            "SELECT 1 FROM _migrations WHERE version = 13",
            libsql::params![],
        )
        .await?;
    if rows.next().await?.is_none() {
        let tx = conn.transaction().await?;
        tx.execute_batch(V13_SCHEMA).await?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (13, 'documents_created_at')",
            libsql::params![],
        )
        .await?;
        tx.commit().await?;
    }

    let mut rows = conn
        .query(
            "SELECT 1 FROM _migrations WHERE version = 14",
            libsql::params![],
        )
        .await?;
    if rows.next().await?.is_none() {
        let tx = conn.transaction().await?;
        tx.execute_batch(V14_SCHEMA).await?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (14, 'robots_bypass_hosts_schema')",
            libsql::params![],
        )
        .await?;
        tx.commit().await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use libsql::Builder;

    #[tokio::test]
    async fn should_create_tables_when_migrations_run() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='documents'",
                libsql::params![],
            )
            .await
            .expect("failed to query sqlite_master");
        assert!(
            rows.next().await.unwrap().is_some(),
            "documents table should exist"
        );

        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='persona'",
                libsql::params![],
            )
            .await
            .expect("failed to query sqlite_master");
        assert!(
            rows.next().await.unwrap().is_some(),
            "persona table should exist"
        );
    }

    #[tokio::test]
    async fn should_be_idempotent_when_migrations_run_twice() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("first migration failed");
        run_migrations(&conn)
            .await
            .expect("second migration should also succeed");
    }

    #[tokio::test]
    async fn should_create_ingest_config_tables_when_migrations_run() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        for table_name in &[
            "ingest_schedule",
            "ingest_section",
            "ingest_source",
            "ingest_run_request",
        ] {
            let mut rows = conn
                .query(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name=?1",
                    libsql::params![table_name],
                )
                .await
                .expect("query failed");
            assert!(
                rows.next().await.unwrap().is_some(),
                "{table_name} table should exist"
            );
        }

        run_migrations(&conn)
            .await
            .expect("second migration run should also succeed");
    }

    #[tokio::test]
    async fn should_create_training_session_table_when_migrations_run() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='training_session'",
                libsql::params![],
            )
            .await
            .expect("query failed");
        assert!(
            rows.next().await.unwrap().is_some(),
            "training_session table should exist"
        );

        run_migrations(&conn)
            .await
            .expect("second migration run should also succeed");
    }

    #[tokio::test]
    async fn should_create_training_message_table_when_migrations_run() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='training_message'",
                libsql::params![],
            )
            .await
            .expect("query failed");
        assert!(
            rows.next().await.unwrap().is_some(),
            "training_message table should exist"
        );

        run_migrations(&conn)
            .await
            .expect("second migration run should also succeed");
    }

    #[tokio::test]
    async fn should_create_training_feedback_table_when_migrations_run() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='training_feedback'",
                libsql::params![],
            )
            .await
            .expect("query failed");
        assert!(
            rows.next().await.unwrap().is_some(),
            "training_feedback table should exist"
        );

        run_migrations(&conn)
            .await
            .expect("second migration run should also succeed");
    }

    #[tokio::test]
    async fn should_create_audit_log_table_when_migrations_run() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='audit_log'",
                libsql::params![],
            )
            .await
            .expect("query failed");
        assert!(
            rows.next().await.unwrap().is_some(),
            "audit_log table should exist"
        );

        run_migrations(&conn)
            .await
            .expect("second migration run should also succeed");
    }

    #[tokio::test]
    async fn should_add_training_redesign_columns_when_migrations_run() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        conn.execute(
            "INSERT INTO training_session (title, notes) VALUES ('s', 'note')",
            libsql::params![],
        )
        .await
        .expect("training_session.notes column should exist");

        conn.execute(
            "INSERT INTO training_message (session_id, question, answer, sources, fell_back, expected_answer, execution_time_ms, source) \
             VALUES (1, 'q', 'a', '[]', 0, 'expected', 42, 'manual')",
            libsql::params![],
        )
        .await
        .expect("training_message new columns should exist");

        run_migrations(&conn)
            .await
            .expect("second migration run should also succeed");
    }

    #[tokio::test]
    async fn should_add_documents_section_column_when_migrations_run() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        conn.execute(
            "INSERT INTO documents (source, source_ref, content, section) VALUES ('scrape', 'https://example.com', 'x', 'news')",
            libsql::params![],
        )
        .await
        .expect("documents.section column should exist");

        run_migrations(&conn)
            .await
            .expect("second migration run should also succeed");
    }

    #[tokio::test]
    async fn should_backfill_section_from_metadata_when_column_was_null() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        // Simulate a pre-V8 scraped document: section column NULL, but the
        // chunker's own metadata already recorded the section name.
        conn.execute(
            r#"INSERT INTO documents (source, source_ref, content, metadata)
               VALUES ('scrape', 'https://example.com', 'x', '{"section":"storia","source_url":"https://example.com"}')"#,
            libsql::params![],
        )
        .await
        .expect("insert failed");

        // A manual upload's metadata never recorded a section — must stay NULL.
        conn.execute(
            r#"INSERT INTO documents (source, source_ref, content, metadata)
               VALUES ('manual', 'doc.pdf', 'y', '{"category":"news","tags":null,"trust_score":0.9}')"#,
            libsql::params![],
        )
        .await
        .expect("insert failed");

        // The first `run_migrations` call already recorded V9 as applied
        // (against an empty table, a no-op) — force it to re-run now that
        // there is data to backfill, exactly as it would on a pre-V8
        // database being migrated for the first time.
        conn.execute(
            "DELETE FROM _migrations WHERE version = 9",
            libsql::params![],
        )
        .await
        .expect("failed to reset migration record");

        run_migrations(&conn)
            .await
            .expect("re-running migrations should apply the backfill");

        let mut rows = conn
            .query(
                "SELECT section FROM documents WHERE source_ref = 'https://example.com'",
                libsql::params![],
            )
            .await
            .expect("query failed");
        let section: Option<String> = rows
            .next()
            .await
            .expect("query failed")
            .expect("row should exist")
            .get(0)
            .expect("failed to read section");
        assert_eq!(section.as_deref(), Some("storia"));

        let mut rows = conn
            .query(
                "SELECT section FROM documents WHERE source_ref = 'doc.pdf'",
                libsql::params![],
            )
            .await
            .expect("query failed");
        let section: Option<String> = rows
            .next()
            .await
            .expect("query failed")
            .expect("row should exist")
            .get(0)
            .expect("failed to read section");
        assert!(section.is_none());
    }

    #[tokio::test]
    async fn should_create_ingest_bookmark_table_when_migrations_run() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='ingest_bookmark'",
                libsql::params![],
            )
            .await
            .expect("query failed");
        assert!(
            rows.next().await.unwrap().is_some(),
            "ingest_bookmark table should exist"
        );

        run_migrations(&conn)
            .await
            .expect("second migration run should also succeed");
    }

    #[tokio::test]
    async fn should_enforce_unique_section_and_source_url_on_ingest_bookmark() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        conn.execute(
            "INSERT INTO ingest_section (name, ordering) VALUES ('delibere', 0)",
            libsql::params![],
        )
        .await
        .expect("insert section failed");

        conn.execute(
            "INSERT INTO ingest_bookmark (section_id, source_url, last_item_ref, last_item_date) \
             VALUES (1, 'https://example.com/delibere', '74', '2026-07-13')",
            libsql::params![],
        )
        .await
        .expect("first bookmark insert should succeed");

        let result = conn
            .execute(
                "INSERT INTO ingest_bookmark (section_id, source_url, last_item_ref, last_item_date) \
                 VALUES (1, 'https://example.com/delibere', '75', '2026-07-20')",
                libsql::params![],
            )
            .await;
        assert!(
            result.is_err(),
            "duplicate (section_id, source_url) should violate the UNIQUE constraint"
        );
    }

    #[tokio::test]
    async fn should_backfill_manual_document_section_from_metadata_category() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        // A manual upload made before this fix: section column NULL, but
        // metadata.category already recorded the section it was uploaded
        // into (the "auto-derive upload metadata" convention).
        conn.execute(
            r#"INSERT INTO documents (source, source_ref, content, metadata)
               VALUES ('manual', 'delibera-74.pdf', 'x', '{"category":"delibere","tags":null,"trust_score":0.9}')"#,
            libsql::params![],
        )
        .await
        .expect("insert failed");

        // A scraped row with no category — must stay NULL (this migration
        // only targets source='manual' rows; V9 already covers scrape).
        conn.execute(
            r#"INSERT INTO documents (source, source_ref, content, metadata)
               VALUES ('scrape', 'https://example.com', 'y', '{"other":"field"}')"#,
            libsql::params![],
        )
        .await
        .expect("insert failed");

        // Re-run to force V11 to apply against data that didn't exist when
        // it was first recorded as applied (same technique as the V9 test).
        conn.execute(
            "DELETE FROM _migrations WHERE version = 11",
            libsql::params![],
        )
        .await
        .expect("failed to reset migration record");

        run_migrations(&conn)
            .await
            .expect("re-running migrations should apply the backfill");

        let mut rows = conn
            .query(
                "SELECT section FROM documents WHERE source_ref = 'delibera-74.pdf'",
                libsql::params![],
            )
            .await
            .expect("query failed");
        let section: Option<String> = rows
            .next()
            .await
            .expect("query failed")
            .expect("row should exist")
            .get(0)
            .expect("failed to read section");
        assert_eq!(section.as_deref(), Some("delibere"));

        let mut rows = conn
            .query(
                "SELECT section FROM documents WHERE source_ref = 'https://example.com'",
                libsql::params![],
            )
            .await
            .expect("query failed");
        let section: Option<String> = rows
            .next()
            .await
            .expect("query failed")
            .expect("row should exist")
            .get(0)
            .expect("failed to read section");
        assert!(section.is_none());
    }

    #[tokio::test]
    async fn should_remap_historical_manual_categories_to_real_section_names() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        for (source_ref, category) in [
            ("74.pdf", "delibera"),
            ("det455.pdf", "determina"),
            ("1113-auser-media-vallesina.md", "civic"),
            ("giunta-consiglio.md", "roster"),
            ("orari.txt", "orari"),
        ] {
            conn.execute(
                &format!(
                    r#"INSERT INTO documents (source, source_ref, content, metadata)
                       VALUES ('manual', '{source_ref}', 'x', '{{"category":"{category}","tags":null,"trust_score":null}}')"#
                ),
                libsql::params![],
            )
            .await
            .expect("insert failed");
        }

        // Force V11 and V12 to re-apply against this freshly-inserted data.
        conn.execute(
            "DELETE FROM _migrations WHERE version IN (11, 12)",
            libsql::params![],
        )
        .await
        .expect("failed to reset migration records");

        run_migrations(&conn)
            .await
            .expect("re-running migrations should apply backfill and remap");

        let expectations = [
            ("74.pdf", Some("delibere")),
            ("det455.pdf", Some("delibere")),
            ("1113-auser-media-vallesina.md", Some("news")),
            ("giunta-consiglio.md", Some("giunta")),
            ("orari.txt", Some("orari")),
        ];
        for (source_ref, expected_section) in expectations {
            let mut rows = conn
                .query(
                    "SELECT section FROM documents WHERE source_ref = ?1",
                    libsql::params![source_ref],
                )
                .await
                .expect("query failed");
            let section: Option<String> = rows
                .next()
                .await
                .expect("query failed")
                .expect("row should exist")
                .get(0)
                .expect("failed to read section");
            assert_eq!(
                section.as_deref(),
                expected_section,
                "unexpected section for {source_ref}"
            );
        }
    }

    #[tokio::test]
    async fn should_add_documents_created_at_column_when_migrations_run() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        // The column itself has no table-level DEFAULT (libsql rejects a
        // non-constant default on ALTER TABLE ADD COLUMN) — `insert_document`
        // sets `created_at` explicitly via `datetime('now')` in its own
        // INSERT statement instead, which this raw INSERT deliberately
        // doesn't do, to confirm the column merely exists and accepts NULL.
        conn.execute(
            "INSERT INTO documents (source, source_ref, content) VALUES ('manual', 'x.pdf', 'y')",
            libsql::params![],
        )
        .await
        .expect("insert failed — the created_at column must exist and be nullable");

        run_migrations(&conn)
            .await
            .expect("second migration run should also succeed");
    }

    #[tokio::test]
    async fn should_create_and_prepopulate_robots_bypass_host_table_when_migrations_run() {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("failed to create in-memory db");
        let conn = db.connect().expect("failed to connect");

        run_migrations(&conn).await.expect("migrations failed");

        let mut rows = conn
            .query("SELECT host FROM robots_bypass_host", libsql::params![])
            .await
            .expect("table must exist");
        let row = rows
            .next()
            .await
            .expect("query failed")
            .expect("the comune's own site must be pre-populated as a bypass host");
        let host: String = row.get(0).unwrap();
        assert_eq!(host, "www.comune.maiolatispontini.an.it");
        assert!(
            rows.next().await.unwrap().is_none(),
            "expected exactly one row"
        );

        run_migrations(&conn)
            .await
            .expect("second migration run should also succeed, without duplicating the seed row");
        let mut rows = conn
            .query("SELECT COUNT(*) FROM robots_bypass_host", libsql::params![])
            .await
            .expect("query failed");
        let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
        assert_eq!(
            count, 1,
            "re-running migrations must not duplicate the seed row"
        );
    }
}
