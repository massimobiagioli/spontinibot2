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
}
