//! Database access layer for Spontini Bot 2.
//!
//! `kb-store` provides a local libSQL database with the `documents` and `persona`
//! tables specified in [STACK.md §3.5](https://github.com/massimobiagioli/spontini-bot-2/blob/main/docs/STACK.md#35-storage--libsql).
//! It is the single shared access layer consumed by both the `backend` and `ingest` crates.
//!
//! # Usage
//!
//! ```no_run
//! use kb_store::{KbStore, NewDocument, DocumentSource};
//! # async fn example() -> kb_store::Result<()> {
//!     let store = KbStore::open("/data/kb.db").await?;
//!     let doc = store.insert_document(NewDocument {
//!         source: DocumentSource::Manual,
//!         source_ref: "file.pdf".into(),
//!         content: "Document content".into(),
//!         metadata: None,
//!         embedding: vec![0.0; 768],
//!     }).await?;
//! #   Ok(())
//! # }
//! ```

pub mod error;
pub(crate) mod migrations;
pub mod types;

pub use error::{KbStoreError, Result};
pub use types::{
    Document, DocumentSource, EMBEDDING_DIM, IngestRunRequest, IngestSchedule, IngestSection,
    IngestSource, NewDocument, NewIngestSchedule, NewIngestSection, NewIngestSource, NewPersona,
    NewTrainingMessage, NewTrainingSession, Persona, RunRequestStatus, ScoredDocument, SourceType,
    TrainingMessage, TrainingSession,
};

use libsql::{Builder, Database, Row};

pub struct KbStore {
    db: Database,
}

impl KbStore {
    pub async fn open(path: &str) -> Result<Self> {
        let db = Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        migrations::run_migrations(&conn).await?;
        Ok(Self { db })
    }

    pub async fn insert_document(&self, doc: NewDocument) -> Result<Document> {
        if doc.embedding.len() != EMBEDDING_DIM {
            return Err(KbStoreError::InvalidDimension {
                expected: EMBEDDING_DIM,
                actual: doc.embedding.len(),
            });
        }

        let conn = self.db.connect()?;
        let blob = f32_slice_to_blob(&doc.embedding);
        let source_str = doc.source.to_string();
        let source_ref = doc.source_ref.clone();
        let content = doc.content.clone();
        let metadata = doc.metadata.clone();

        conn.execute(
            "INSERT INTO documents (source, source_ref, content, metadata, embedding) VALUES (?1, ?2, ?3, ?4, ?5)",
            libsql::params![
                source_str,
                source_ref,
                content,
                metadata,
                libsql::Value::Blob(blob),
            ],
        )
        .await?;

        let id = conn.last_insert_rowid();

        Ok(Document {
            id,
            source: doc.source,
            source_ref: doc.source_ref,
            content: doc.content,
            metadata: doc.metadata,
            embedding: Some(doc.embedding),
        })
    }

    pub async fn get_document(&self, id: i64) -> Result<Option<Document>> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query("SELECT id, source, source_ref, content, metadata, embedding FROM documents WHERE id = ?1", libsql::params![id])
            .await?;
        match rows.next().await? {
            Some(row) => Ok(Some(row_to_document(&row)?)),
            None => Ok(None),
        }
    }

    pub async fn get_documents_by_source(
        &self,
        source: DocumentSource,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Document>> {
        let conn = self.db.connect()?;
        let source_str = source.to_string();
        let mut rows = conn
            .query(
                "SELECT id, source, source_ref, content, metadata, embedding FROM documents WHERE source = ?1 ORDER BY id DESC LIMIT ?2 OFFSET ?3",
                libsql::params![source_str, limit, offset],
            )
            .await?;
        let mut docs = Vec::new();
        while let Some(row) = rows.next().await? {
            docs.push(row_to_document(&row)?);
        }
        Ok(docs)
    }

    pub async fn search_similar(
        &self,
        query_embedding: &[f32],
        top_k: i64,
        min_score: f64,
    ) -> Result<Vec<ScoredDocument>> {
        if query_embedding.len() != EMBEDDING_DIM {
            return Err(KbStoreError::InvalidDimension {
                expected: EMBEDDING_DIM,
                actual: query_embedding.len(),
            });
        }

        let conn = self.db.connect()?;
        let blob = f32_slice_to_blob(query_embedding);
        let query = "SELECT id, source, source_ref, content, metadata, embedding, \
                          1 - vector_distance_cos(embedding, vector32(?1)) AS similarity \
                     FROM documents \
                     WHERE embedding IS NOT NULL \
                       AND (1 - vector_distance_cos(embedding, vector32(?1))) >= ?3 \
                     ORDER BY similarity DESC \
                     LIMIT ?2";

        let mut rows = conn
            .query(
                query,
                libsql::params![libsql::Value::Blob(blob), top_k, min_score,],
            )
            .await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let document = row_to_document(&row)?;
            let similarity = row.get::<f64>(6)?;
            results.push(ScoredDocument {
                document,
                similarity,
            });
        }
        Ok(results)
    }

    pub async fn delete_document(&self, id: i64) -> Result<bool> {
        let conn = self.db.connect()?;
        let rows_affected = conn
            .execute("DELETE FROM documents WHERE id = ?1", libsql::params![id])
            .await?;
        Ok(rows_affected > 0)
    }

    pub async fn insert_persona(&self, persona: NewPersona, activate: bool) -> Result<Persona> {
        let conn = self.db.connect()?;
        let tx = conn.transaction().await?;

        let mut rows = tx
            .query(
                "SELECT COALESCE(MAX(version), 0) + 1 FROM persona WHERE name = ?1",
                libsql::params![persona.name.clone()],
            )
            .await?;
        let version: i32 = rows
            .next()
            .await?
            .map_or(1, |r| r.get::<i32>(0).unwrap_or(1));

        if activate {
            tx.execute(
                "UPDATE persona SET is_active = 0 WHERE is_active = 1",
                libsql::params![],
            )
            .await?;
        }

        tx.execute(
            "INSERT INTO persona (version, name, system_prompt, tone, fallback_message, is_active, created_by) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            libsql::params![
                version,
                persona.name.clone(),
                persona.system_prompt.clone(),
                persona.tone.clone(),
                persona.fallback_message.clone(),
                activate as i32,
                persona.created_by.clone(),
            ],
        )
        .await?;

        let id = tx.last_insert_rowid();

        let mut rows = tx
            .query(
                "SELECT created_at FROM persona WHERE id = ?1",
                libsql::params![id],
            )
            .await?;
        let created_at: String = rows
            .next()
            .await?
            .map_or_else(|| "".into(), |r| r.get::<String>(0).unwrap_or_default());

        tx.commit().await?;

        Ok(Persona {
            id,
            version,
            name: persona.name,
            system_prompt: persona.system_prompt,
            tone: persona.tone,
            fallback_message: persona.fallback_message,
            is_active: activate,
            created_at,
            created_by: persona.created_by,
        })
    }

    pub async fn get_active_persona(&self) -> Result<Option<Persona>> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT id, version, name, system_prompt, tone, fallback_message, is_active, created_at, created_by \
                 FROM persona WHERE is_active = 1 LIMIT 1",
                libsql::params![],
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(Some(row_to_persona(&row)?)),
            None => Ok(None),
        }
    }

    pub async fn get_persona(&self, id: i64) -> Result<Option<Persona>> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT id, version, name, system_prompt, tone, fallback_message, is_active, created_at, created_by \
                 FROM persona WHERE id = ?1",
                libsql::params![id],
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(Some(row_to_persona(&row)?)),
            None => Ok(None),
        }
    }

    pub async fn get_persona_versions(&self, name: &str) -> Result<Vec<Persona>> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT id, version, name, system_prompt, tone, fallback_message, is_active, created_at, created_by \
                 FROM persona WHERE name = ?1 ORDER BY version DESC",
                libsql::params![name],
            )
            .await?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            results.push(row_to_persona(&row)?);
        }
        Ok(results)
    }

    pub async fn activate_persona(&self, id: i64) -> Result<()> {
        let conn = self.db.connect()?;

        let exists = conn
            .query("SELECT 1 FROM persona WHERE id = ?1", libsql::params![id])
            .await?
            .next()
            .await?
            .is_some();
        if !exists {
            return Err(KbStoreError::NotFound(format!("persona {id}")));
        }

        let tx = conn.transaction().await?;
        tx.execute(
            "UPDATE persona SET is_active = 0 WHERE is_active = 1",
            libsql::params![],
        )
        .await?;
        tx.execute(
            "UPDATE persona SET is_active = 1 WHERE id = ?1",
            libsql::params![id],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_schedule(&self) -> Result<Option<IngestSchedule>> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT cron_expr, enabled, updated_at FROM ingest_schedule WHERE id = 1",
                libsql::params![],
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(Some(IngestSchedule {
                cron_expr: row.get::<String>(0)?,
                enabled: row.get::<i32>(1)? != 0,
                updated_at: row.get::<String>(2)?,
            })),
            None => Ok(None),
        }
    }

    pub async fn upsert_schedule(&self, schedule: NewIngestSchedule) -> Result<IngestSchedule> {
        let conn = self.db.connect()?;
        conn.execute(
            "INSERT OR REPLACE INTO ingest_schedule (id, cron_expr, enabled) VALUES (1, ?1, ?2)",
            libsql::params![schedule.cron_expr, schedule.enabled as i32],
        )
        .await?;
        self.get_schedule()
            .await?
            .ok_or_else(|| KbStoreError::Migration("schedule not found after upsert".into()))
    }

    pub async fn list_sections(&self) -> Result<Vec<IngestSection>> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT id, name, ordering, created_at FROM ingest_section ORDER BY ordering ASC, id ASC",
                libsql::params![],
            )
            .await?;
        let mut sections = Vec::new();
        while let Some(row) = rows.next().await? {
            sections.push(IngestSection {
                id: row.get::<i64>(0)?,
                name: row.get::<String>(1)?,
                ordering: row.get::<i32>(2)?,
                created_at: row.get::<String>(3)?,
            });
        }
        Ok(sections)
    }

    pub async fn upsert_section(&self, section: NewIngestSection) -> Result<IngestSection> {
        let conn = self.db.connect()?;
        conn.execute(
            "INSERT INTO ingest_section (name, ordering) VALUES (?1, ?2)",
            libsql::params![section.name, section.ordering],
        )
        .await?;
        let id = conn.last_insert_rowid();
        let mut rows = conn
            .query(
                "SELECT id, name, ordering, created_at FROM ingest_section WHERE id = ?1",
                libsql::params![id],
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(IngestSection {
                id: row.get::<i64>(0)?,
                name: row.get::<String>(1)?,
                ordering: row.get::<i32>(2)?,
                created_at: row.get::<String>(3)?,
            }),
            None => Err(KbStoreError::Migration(
                "section not found after insert".into(),
            )),
        }
    }

    pub async fn delete_section(&self, id: i64) -> Result<bool> {
        let conn = self.db.connect()?;
        let rows_affected = conn
            .execute(
                "DELETE FROM ingest_section WHERE id = ?1",
                libsql::params![id],
            )
            .await?;
        Ok(rows_affected > 0)
    }

    pub async fn list_sources_by_section(&self, section_id: i64) -> Result<Vec<IngestSource>> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT id, section_id, source_type, url, enabled, created_at FROM ingest_source WHERE section_id = ?1 ORDER BY id ASC",
                libsql::params![section_id],
            )
            .await?;
        let mut sources = Vec::new();
        while let Some(row) = rows.next().await? {
            let source_type_str: String = row.get::<String>(2)?;
            let source_type = source_type_str
                .parse::<SourceType>()
                .map_err(|e| KbStoreError::Migration(format!("invalid source_type in db: {e}")))?;
            sources.push(IngestSource {
                id: row.get::<i64>(0)?,
                section_id: row.get::<i64>(1)?,
                source_type,
                url: row.get::<String>(3)?,
                enabled: row.get::<i32>(4)? != 0,
                created_at: row.get::<String>(5)?,
            });
        }
        Ok(sources)
    }

    pub async fn upsert_source(&self, source: NewIngestSource) -> Result<IngestSource> {
        let conn = self.db.connect()?;
        let source_type_str = source.source_type.to_string();
        conn.execute(
            "INSERT INTO ingest_source (section_id, source_type, url, enabled) VALUES (?1, ?2, ?3, ?4)",
            libsql::params![source.section_id, source_type_str, source.url, source.enabled as i32],
        )
        .await?;
        let id = conn.last_insert_rowid();
        let mut rows = conn
            .query(
                "SELECT id, section_id, source_type, url, enabled, created_at FROM ingest_source WHERE id = ?1",
                libsql::params![id],
            )
            .await?;
        match rows.next().await? {
            Some(row) => {
                let source_type_str: String = row.get::<String>(2)?;
                let source_type = source_type_str.parse::<SourceType>().map_err(|e| {
                    KbStoreError::Migration(format!("invalid source_type in db: {e}"))
                })?;
                Ok(IngestSource {
                    id: row.get::<i64>(0)?,
                    section_id: row.get::<i64>(1)?,
                    source_type,
                    url: row.get::<String>(3)?,
                    enabled: row.get::<i32>(4)? != 0,
                    created_at: row.get::<String>(5)?,
                })
            }
            None => Err(KbStoreError::Migration(
                "source not found after insert".into(),
            )),
        }
    }

    pub async fn delete_source(&self, id: i64) -> Result<bool> {
        let conn = self.db.connect()?;
        let rows_affected = conn
            .execute(
                "DELETE FROM ingest_source WHERE id = ?1",
                libsql::params![id],
            )
            .await?;
        Ok(rows_affected > 0)
    }

    pub async fn request_run(&self) -> Result<IngestRunRequest> {
        let conn = self.db.connect()?;
        conn.execute(
            "INSERT INTO ingest_run_request DEFAULT VALUES",
            libsql::params![],
        )
        .await?;
        let id = conn.last_insert_rowid();
        let mut rows = conn
            .query(
                "SELECT id, requested_at, status FROM ingest_run_request WHERE id = ?1",
                libsql::params![id],
            )
            .await?;
        match rows.next().await? {
            Some(row) => {
                let status_str: String = row.get::<String>(2)?;
                let status = status_str
                    .parse::<RunRequestStatus>()
                    .map_err(|e| KbStoreError::Migration(format!("invalid status in db: {e}")))?;
                Ok(IngestRunRequest {
                    id: row.get::<i64>(0)?,
                    requested_at: row.get::<String>(1)?,
                    status,
                })
            }
            None => Err(KbStoreError::Migration(
                "run request not found after insert".into(),
            )),
        }
    }

    pub async fn get_run_request(&self, id: i64) -> Result<Option<IngestRunRequest>> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT id, requested_at, status FROM ingest_run_request WHERE id = ?1",
                libsql::params![id],
            )
            .await?;
        match rows.next().await? {
            Some(row) => {
                let status_str: String = row.get::<String>(2)?;
                let status = status_str
                    .parse::<RunRequestStatus>()
                    .map_err(|e| KbStoreError::Migration(format!("invalid status in db: {e}")))?;
                Ok(Some(IngestRunRequest {
                    id: row.get::<i64>(0)?,
                    requested_at: row.get::<String>(1)?,
                    status,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn consume_run_request(&self) -> Result<Option<IngestRunRequest>> {
        let conn = self.db.connect()?;
        let tx = conn.transaction().await?;
        let mut rows = tx
            .query(
                "SELECT id, requested_at, status FROM ingest_run_request WHERE status = 'pending' ORDER BY id ASC LIMIT 1",
                libsql::params![],
            )
            .await?;
        let row = match rows.next().await? {
            Some(row) => row,
            None => {
                tx.rollback().await?;
                return Ok(None);
            }
        };
        let id = row.get::<i64>(0)?;
        let requested_at = row.get::<String>(1)?;
        tx.execute(
            "UPDATE ingest_run_request SET status = 'running' WHERE id = ?1",
            libsql::params![id],
        )
        .await?;
        tx.commit().await?;
        Ok(Some(IngestRunRequest {
            id,
            requested_at,
            status: RunRequestStatus::Running,
        }))
    }

    pub async fn create_training_session(
        &self,
        session: NewTrainingSession,
    ) -> Result<TrainingSession> {
        let conn = self.db.connect()?;
        conn.execute(
            "INSERT INTO training_session (title, created_by) VALUES (?1, ?2)",
            libsql::params![session.title, session.created_by],
        )
        .await?;
        let id = conn.last_insert_rowid();
        let mut rows = conn
            .query(
                "SELECT id, title, created_at, created_by, closed_at FROM training_session WHERE id = ?1",
                libsql::params![id],
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(TrainingSession {
                id: row.get::<i64>(0)?,
                title: row.get::<String>(1)?,
                created_at: row.get::<String>(2)?,
                created_by: row.get::<Option<String>>(3)?,
                closed_at: row.get::<Option<String>>(4)?,
            }),
            None => Err(KbStoreError::Migration(
                "training session not found after insert".into(),
            )),
        }
    }

    pub async fn list_training_sessions(&self) -> Result<Vec<TrainingSession>> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT id, title, created_at, created_by, closed_at FROM training_session ORDER BY created_at DESC, id DESC",
                libsql::params![],
            )
            .await?;
        let mut sessions = Vec::new();
        while let Some(row) = rows.next().await? {
            sessions.push(TrainingSession {
                id: row.get::<i64>(0)?,
                title: row.get::<String>(1)?,
                created_at: row.get::<String>(2)?,
                created_by: row.get::<Option<String>>(3)?,
                closed_at: row.get::<Option<String>>(4)?,
            });
        }
        Ok(sessions)
    }

    pub async fn get_training_session(&self, id: i64) -> Result<Option<TrainingSession>> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT id, title, created_at, created_by, closed_at FROM training_session WHERE id = ?1",
                libsql::params![id],
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(Some(TrainingSession {
                id: row.get::<i64>(0)?,
                title: row.get::<String>(1)?,
                created_at: row.get::<String>(2)?,
                created_by: row.get::<Option<String>>(3)?,
                closed_at: row.get::<Option<String>>(4)?,
            })),
            None => Ok(None),
        }
    }

    pub async fn close_training_session(&self, id: i64) -> Result<bool> {
        let conn = self.db.connect()?;
        let rows_affected = conn
            .execute(
                "UPDATE training_session SET closed_at = datetime('now') WHERE id = ?1 AND closed_at IS NULL",
                libsql::params![id],
            )
            .await?;
        Ok(rows_affected > 0)
    }

    pub async fn create_training_message(
        &self,
        message: NewTrainingMessage,
    ) -> Result<TrainingMessage> {
        let conn = self.db.connect()?;
        conn.execute(
            "INSERT INTO training_message (session_id, question, answer, sources, fell_back) VALUES (?1, ?2, ?3, ?4, ?5)",
            libsql::params![
                message.session_id,
                message.question,
                message.answer,
                message.sources,
                message.fell_back as i32,
            ],
        )
        .await?;
        let id = conn.last_insert_rowid();
        let mut rows = conn
            .query(
                "SELECT id, session_id, question, answer, sources, fell_back, created_at FROM training_message WHERE id = ?1",
                libsql::params![id],
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(TrainingMessage {
                id: row.get::<i64>(0)?,
                session_id: row.get::<i64>(1)?,
                question: row.get::<String>(2)?,
                answer: row.get::<String>(3)?,
                sources: row.get::<String>(4)?,
                fell_back: row.get::<i64>(5)? != 0,
                created_at: row.get::<String>(6)?,
            }),
            None => Err(KbStoreError::Migration(
                "training message not found after insert".into(),
            )),
        }
    }

    pub async fn list_training_messages(&self, session_id: i64) -> Result<Vec<TrainingMessage>> {
        let conn = self.db.connect()?;
        let mut rows = conn
            .query(
                "SELECT id, session_id, question, answer, sources, fell_back, created_at FROM training_message WHERE session_id = ?1 ORDER BY created_at ASC, id ASC",
                libsql::params![session_id],
            )
            .await?;
        let mut messages = Vec::new();
        while let Some(row) = rows.next().await? {
            messages.push(TrainingMessage {
                id: row.get::<i64>(0)?,
                session_id: row.get::<i64>(1)?,
                question: row.get::<String>(2)?,
                answer: row.get::<String>(3)?,
                sources: row.get::<String>(4)?,
                fell_back: row.get::<i64>(5)? != 0,
                created_at: row.get::<String>(6)?,
            });
        }
        Ok(messages)
    }

    pub async fn complete_run(&self, id: i64, status: RunRequestStatus) -> Result<()> {
        let conn = self.db.connect()?;
        let exists = conn
            .query(
                "SELECT 1 FROM ingest_run_request WHERE id = ?1",
                libsql::params![id],
            )
            .await?
            .next()
            .await?
            .is_some();
        if !exists {
            return Err(KbStoreError::NotFound(format!("run request {id}")));
        }
        let status_str = status.to_string();
        conn.execute(
            "UPDATE ingest_run_request SET status = ?2 WHERE id = ?1",
            libsql::params![id, status_str],
        )
        .await?;
        Ok(())
    }
}

pub(crate) fn f32_slice_to_blob(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

pub(crate) fn blob_to_f32_vec(blob: Vec<u8>) -> Result<Vec<f32>> {
    if !blob.len().is_multiple_of(4) {
        return Err(KbStoreError::InvalidDimension {
            expected: EMBEDDING_DIM,
            actual: blob.len() / 4,
        });
    }
    Ok(blob
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

pub(crate) fn row_to_document(row: &Row) -> Result<Document> {
    let id = row.get::<i64>(0)?;
    let source_str: String = row.get::<String>(1)?;
    let source = source_str
        .parse::<DocumentSource>()
        .map_err(|e| KbStoreError::Migration(format!("invalid source in db: {e}")))?;
    Ok(Document {
        id,
        source,
        source_ref: row.get::<String>(2)?,
        content: row.get::<String>(3)?,
        metadata: row.get::<Option<String>>(4)?,
        embedding: row
            .get::<Option<Vec<u8>>>(5)?
            .map(blob_to_f32_vec)
            .transpose()?,
    })
}

pub(crate) fn row_to_persona(row: &Row) -> Result<Persona> {
    Ok(Persona {
        id: row.get::<i64>(0)?,
        version: row.get::<i32>(1)?,
        name: row.get::<String>(2)?,
        system_prompt: row.get::<String>(3)?,
        tone: row.get::<Option<String>>(4)?,
        fallback_message: row.get::<Option<String>>(5)?,
        is_active: row.get::<i32>(6)? != 0,
        created_at: row.get::<String>(7)?,
        created_by: row.get::<Option<String>>(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static DB_COUNTER: AtomicU32 = AtomicU32::new(1);

    fn temp_db_path() -> String {
        let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir();
        dir.join(format!("kb_store_test_{n}.db"))
            .to_string_lossy()
            .into_owned()
    }

    fn sample_new_document(embedding: Vec<f32>) -> NewDocument {
        NewDocument {
            source: DocumentSource::Manual,
            source_ref: "test.pdf".into(),
            content: "Test content".into(),
            metadata: Some(r#"{"tags":["test"]}"#.into()),
            embedding,
        }
    }

    #[tokio::test]
    async fn should_open_database_when_path_given() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let conn = store.db.connect().expect("failed to connect");
        let mut rows = conn
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='documents'",
                libsql::params![],
            )
            .await
            .expect("query failed");
        assert!(rows.next().await.unwrap().is_some(), "table should exist");
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_insert_document_when_valid_embedding_provided() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let embedding = vec![0.1f32; EMBEDDING_DIM];
        let new_doc = sample_new_document(embedding.clone());

        let doc = store
            .insert_document(new_doc)
            .await
            .expect("failed to insert");

        assert_eq!(doc.source, DocumentSource::Manual);
        assert_eq!(doc.source_ref, "test.pdf");
        assert_eq!(doc.content, "Test content");
        assert!(
            doc.embedding
                .as_ref()
                .is_some_and(|e| e.len() == EMBEDDING_DIM)
        );
        assert!(doc.id > 0, "should auto-generate an id");
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_reject_document_when_wrong_dimension() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let embedding = vec![0.1f32; 512];
        let new_doc = sample_new_document(embedding);

        let result = store.insert_document(new_doc).await;
        assert!(matches!(result, Err(KbStoreError::InvalidDimension { .. })));
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn should_convert_f32_slice_to_blob_and_back() {
        let original: Vec<f32> = vec![0.1, 0.2, 0.3, 0.4, -0.5, 0.6, 0.7, 0.8];
        let blob = f32_slice_to_blob(&original);
        let reconstructed = blob_to_f32_vec(blob).expect("blob conversion failed");
        assert_eq!(reconstructed.len(), original.len());
        for (a, b) in original.iter().zip(reconstructed.iter()) {
            assert!(
                (a - b).abs() < f32::EPSILON,
                "expected {a}, got {b} at index"
            );
        }
    }

    #[test]
    fn should_reject_invalid_blob_length() {
        let bad_blob = vec![0u8, 1, 2];
        let result = blob_to_f32_vec(bad_blob);
        assert!(matches!(result, Err(KbStoreError::InvalidDimension { .. })));
    }

    #[tokio::test]
    async fn should_return_document_when_get_by_existing_id() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let embedding = vec![0.1f32; EMBEDDING_DIM];
        let inserted = store
            .insert_document(sample_new_document(embedding))
            .await
            .expect("failed to insert");

        let found = store
            .get_document(inserted.id)
            .await
            .expect("failed to get");

        assert_eq!(found, Some(inserted));
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_none_when_get_by_missing_id() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let result = store.get_document(999).await.expect("failed to query");
        assert!(result.is_none());
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_documents_filtered_by_source() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let embedding = vec![0.1f32; EMBEDDING_DIM];

        let mut scrape_doc = sample_new_document(embedding.clone());
        scrape_doc.source = DocumentSource::Scrape;
        let s1 = store
            .insert_document(scrape_doc)
            .await
            .expect("insert failed");

        let api_doc = NewDocument {
            source: DocumentSource::Api,
            ..sample_new_document(embedding.clone())
        };
        store.insert_document(api_doc).await.expect("insert failed");

        let mut scrape2 = sample_new_document(embedding);
        scrape2.source = DocumentSource::Scrape;
        let s2 = store.insert_document(scrape2).await.expect("insert failed");

        let results = store
            .get_documents_by_source(DocumentSource::Scrape, 10, 0)
            .await
            .expect("query failed");

        assert_eq!(results.len(), 2);
        let ids: Vec<i64> = results.iter().map(|d| d.id).collect();
        assert!(ids.contains(&s1.id));
        assert!(ids.contains(&s2.id));
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_similar_documents_when_searching() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");

        let vec_a: Vec<f32> = (0..EMBEDDING_DIM).map(|i| i as f32 * 0.001).collect();
        let vec_b: Vec<f32> = (0..EMBEDDING_DIM)
            .map(|i| i as f32 * 0.001 + 10.0)
            .collect();

        store
            .insert_document(NewDocument {
                source_ref: "doc_a".into(),
                ..sample_new_document(vec_a.clone())
            })
            .await
            .expect("insert A failed");
        store
            .insert_document(NewDocument {
                source_ref: "doc_b".into(),
                ..sample_new_document(vec_b.clone())
            })
            .await
            .expect("insert B failed");

        let results = store
            .search_similar(&vec_a, 5, -1.0)
            .await
            .expect("search failed");
        assert!(!results.is_empty(), "should return at least one result");
        assert!(
            results[0].document.source_ref == "doc_a" || results[0].document.source_ref == "doc_b",
            "first result should be one of the inserted docs"
        );
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_empty_when_no_matching_documents() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let embedding = vec![0.1f32; EMBEDDING_DIM];
        store
            .insert_document(sample_new_document(embedding))
            .await
            .expect("insert failed");

        let results = store
            .search_similar(&[0.1f32; EMBEDDING_DIM], 5, 2.0)
            .await
            .expect("search failed");
        assert!(
            results.is_empty(),
            "should be empty — min_score 2.0 exceeds max cosine similarity 1.0"
        );
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_delete_document_when_exists() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let embedding = vec![0.1f32; EMBEDDING_DIM];
        let inserted = store
            .insert_document(sample_new_document(embedding))
            .await
            .expect("insert failed");

        let deleted = store
            .delete_document(inserted.id)
            .await
            .expect("delete failed");
        assert!(deleted);

        let found = store.get_document(inserted.id).await.expect("query failed");
        assert!(found.is_none());
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_false_when_deleting_missing_document() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let result = store
            .delete_document(999)
            .await
            .expect("delete query failed");
        assert!(!result);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    fn sample_new_persona(name: &str) -> NewPersona {
        NewPersona {
            name: name.into(),
            system_prompt: "You are helpful.".into(),
            tone: Some("warm".into()),
            fallback_message: Some("Non lo so".into()),
            created_by: Some("admin".into()),
        }
    }

    #[tokio::test]
    async fn should_insert_persona_with_incrementing_version() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");

        let p1 = store
            .insert_persona(sample_new_persona("gaspare"), false)
            .await
            .expect("insert failed");
        assert_eq!(p1.version, 1);

        let p2 = store
            .insert_persona(sample_new_persona("gaspare"), false)
            .await
            .expect("insert failed");
        assert_eq!(p2.version, 2);

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_have_one_active_persona_when_inserting_with_activate_true() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");

        let p1 = store
            .insert_persona(sample_new_persona("gaspare"), true)
            .await
            .expect("insert failed");
        assert!(p1.is_active);

        let p2 = store
            .insert_persona(sample_new_persona("giovanni"), true)
            .await
            .expect("insert failed");
        assert!(p2.is_active);

        let prev = store.get_persona(p1.id).await.expect("query failed");
        assert!(prev.is_some_and(|p| !p.is_active));

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_none_when_no_active_persona() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        store
            .insert_persona(sample_new_persona("inactive"), false)
            .await
            .expect("insert failed");

        let result = store.get_active_persona().await.expect("query failed");
        assert!(result.is_none());
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_active_persona_when_exists() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let inserted = store
            .insert_persona(sample_new_persona("gaspare"), true)
            .await
            .expect("insert failed");

        let active = store
            .get_active_persona()
            .await
            .expect("query failed")
            .expect("should have active persona");
        assert_eq!(active.id, inserted.id);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_all_versions_when_querying_by_name() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        store
            .insert_persona(sample_new_persona("gaspare"), false)
            .await
            .expect("insert failed");
        store
            .insert_persona(sample_new_persona("gaspare"), false)
            .await
            .expect("insert failed");

        let versions = store
            .get_persona_versions("gaspare")
            .await
            .expect("query failed");
        assert_eq!(versions.len(), 2);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_activate_persona_and_deactivate_others() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let p1 = store
            .insert_persona(sample_new_persona("gaspare"), true)
            .await
            .expect("insert failed");
        let p2 = store
            .insert_persona(sample_new_persona("giovanni"), false)
            .await
            .expect("insert failed");

        store
            .activate_persona(p2.id)
            .await
            .expect("activate failed");

        let p1_after = store.get_persona(p1.id).await.expect("query failed");
        assert!(p1_after.is_some_and(|p| !p.is_active));
        let p2_after = store.get_persona(p2.id).await.expect("query failed");
        assert!(p2_after.is_some_and(|p| p.is_active));
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_error_when_activating_nonexistent_persona() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let result = store.activate_persona(999).await;
        assert!(matches!(result, Err(KbStoreError::NotFound(_))));
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_none_when_no_schedule() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let result = store.get_schedule().await.expect("query failed");
        assert!(result.is_none());
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_upsert_schedule_and_return_it() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let schedule = store
            .upsert_schedule(NewIngestSchedule {
                cron_expr: "0 */4 * * *".into(),
                enabled: true,
            })
            .await
            .expect("upsert failed");
        assert_eq!(schedule.cron_expr, "0 */4 * * *");
        assert!(schedule.enabled);

        let fetched = store.get_schedule().await.expect("query failed");
        assert!(fetched.is_some_and(|s| s.cron_expr == "0 */4 * * *" && s.enabled));

        store
            .upsert_schedule(NewIngestSchedule {
                cron_expr: "30 2 * * *".into(),
                enabled: false,
            })
            .await
            .expect("second upsert failed");
        let fetched = store.get_schedule().await.expect("query failed");
        assert!(fetched.is_some_and(|s| s.cron_expr == "30 2 * * *" && !s.enabled));

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_list_sections_in_ordering_asc() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let s2 = store
            .upsert_section(NewIngestSection {
                name: "news".into(),
                ordering: 20,
            })
            .await
            .expect("insert failed");
        let s1 = store
            .upsert_section(NewIngestSection {
                name: "sport".into(),
                ordering: 10,
            })
            .await
            .expect("insert failed");

        let sections = store.list_sections().await.expect("query failed");
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].id, s1.id);
        assert_eq!(sections[0].name, "sport");
        assert_eq!(sections[1].id, s2.id);
        assert_eq!(sections[1].name, "news");

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_false_when_deleting_missing_section() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let result = store.delete_section(999).await.expect("delete failed");
        assert!(!result);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_list_sources_for_section() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let section = store
            .upsert_section(NewIngestSection {
                name: "sport".into(),
                ordering: 0,
            })
            .await
            .expect("insert failed");
        let s1 = store
            .upsert_source(NewIngestSource {
                section_id: section.id,
                source_type: SourceType::Scrape,
                url: "https://example.com/a".into(),
                enabled: true,
            })
            .await
            .expect("insert failed");
        let s2 = store
            .upsert_source(NewIngestSource {
                section_id: section.id,
                source_type: SourceType::Scrape,
                url: "https://example.com/b".into(),
                enabled: false,
            })
            .await
            .expect("insert failed");

        let sources = store
            .list_sources_by_section(section.id)
            .await
            .expect("query failed");
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].id, s1.id);
        assert_eq!(sources[1].id, s2.id);

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_insert_scrape_and_api_source_types() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let section = store
            .upsert_section(NewIngestSection {
                name: "news".into(),
                ordering: 0,
            })
            .await
            .expect("insert failed");
        let scrape = store
            .upsert_source(NewIngestSource {
                section_id: section.id,
                source_type: SourceType::Scrape,
                url: "https://example.com".into(),
                enabled: true,
            })
            .await
            .expect("insert failed");
        let api = store
            .upsert_source(NewIngestSource {
                section_id: section.id,
                source_type: SourceType::Api,
                url: "https://api.example.com".into(),
                enabled: false,
            })
            .await
            .expect("insert failed");

        assert_eq!(scrape.source_type, SourceType::Scrape);
        assert_eq!(api.source_type, SourceType::Api);

        let sources = store
            .list_sources_by_section(section.id)
            .await
            .expect("query failed");
        assert_eq!(sources.len(), 2);
        assert!(sources.iter().any(|s| s.source_type == SourceType::Scrape));
        assert!(sources.iter().any(|s| s.source_type == SourceType::Api));

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_false_when_deleting_missing_source() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let result = store.delete_source(999).await.expect("delete failed");
        assert!(!result);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_delete_section_and_cascade_delete_sources() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let conn = store.db.connect().expect("failed to connect");
        conn.execute_batch("PRAGMA foreign_keys = ON")
            .await
            .expect("failed to enable FK");
        drop(conn);

        let section = store
            .upsert_section(NewIngestSection {
                name: "sport".into(),
                ordering: 0,
            })
            .await
            .expect("insert failed");
        store
            .upsert_source(NewIngestSource {
                section_id: section.id,
                source_type: SourceType::Scrape,
                url: "https://example.com".into(),
                enabled: true,
            })
            .await
            .expect("insert source failed");

        let sources = store
            .list_sources_by_section(section.id)
            .await
            .expect("query failed");
        assert_eq!(sources.len(), 1);

        let deleted = store
            .delete_section(section.id)
            .await
            .expect("delete failed");
        assert!(deleted);

        let sources = store
            .list_sources_by_section(section.id)
            .await
            .expect("query failed");
        assert!(sources.is_empty());

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_request_run_and_return_pending() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let request = store.request_run().await.expect("request_run failed");
        assert!(request.id > 0);
        assert_eq!(request.status, RunRequestStatus::Pending);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_consume_first_pending_run() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let r1 = store.request_run().await.expect("request_run failed");
        let _r2 = store.request_run().await.expect("request_run failed");

        let consumed = store
            .consume_run_request()
            .await
            .expect("consume failed")
            .expect("should have a pending run");
        assert_eq!(consumed.id, r1.id);
        assert_eq!(consumed.status, RunRequestStatus::Running);

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_none_when_no_pending_run() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let result = store.consume_run_request().await.expect("consume failed");
        assert!(result.is_none());
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_complete_run_with_done_status() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let request = store.request_run().await.expect("request_run failed");
        let _consumed = store
            .consume_run_request()
            .await
            .expect("consume failed")
            .expect("should have a pending run");

        store
            .complete_run(request.id, RunRequestStatus::Done)
            .await
            .expect("complete_run failed");

        let conn = store.db.connect().expect("failed to connect");
        let mut rows = conn
            .query(
                "SELECT status FROM ingest_run_request WHERE id = ?1",
                libsql::params![request.id],
            )
            .await
            .expect("query failed");
        let status_str: String = rows.next().await.unwrap().unwrap().get(0).unwrap();
        assert_eq!(status_str, "done");

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_error_when_completing_missing_run() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let result = store.complete_run(999, RunRequestStatus::Done).await;
        assert!(matches!(result, Err(KbStoreError::NotFound(_))));
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_get_run_request_with_pending_status_right_after_request() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let request = store.request_run().await.expect("request_run failed");

        let fetched = store
            .get_run_request(request.id)
            .await
            .expect("get_run_request failed")
            .expect("should find the request");

        assert_eq!(fetched.id, request.id);
        assert_eq!(fetched.status, RunRequestStatus::Pending);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_get_run_request_with_running_status_after_consume() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let request = store.request_run().await.expect("request_run failed");
        store
            .consume_run_request()
            .await
            .expect("consume failed")
            .expect("should have a pending run");

        let fetched = store
            .get_run_request(request.id)
            .await
            .expect("get_run_request failed")
            .expect("should find the request");

        assert_eq!(fetched.status, RunRequestStatus::Running);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_get_run_request_with_done_status_after_complete() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let request = store.request_run().await.expect("request_run failed");
        store
            .consume_run_request()
            .await
            .expect("consume failed")
            .expect("should have a pending run");
        store
            .complete_run(request.id, RunRequestStatus::Done)
            .await
            .expect("complete_run failed");

        let fetched = store
            .get_run_request(request.id)
            .await
            .expect("get_run_request failed")
            .expect("should find the request");

        assert_eq!(fetched.status, RunRequestStatus::Done);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_none_when_getting_unknown_run_request() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let result = store
            .get_run_request(999)
            .await
            .expect("get_run_request failed");
        assert!(result.is_none());
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_create_training_session_as_open() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");

        let session = store
            .create_training_session(NewTrainingSession {
                title: "Sessione di prova".into(),
                created_by: Some("operator1".into()),
            })
            .await
            .expect("create_training_session failed");

        assert!(session.id > 0);
        assert_eq!(session.title, "Sessione di prova");
        assert_eq!(session.created_by.as_deref(), Some("operator1"));
        assert!(session.closed_at.is_none());
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_list_training_sessions_newest_first() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");

        let first = store
            .create_training_session(NewTrainingSession {
                title: "First".into(),
                created_by: None,
            })
            .await
            .expect("create failed");
        let second = store
            .create_training_session(NewTrainingSession {
                title: "Second".into(),
                created_by: None,
            })
            .await
            .expect("create failed");

        let sessions = store
            .list_training_sessions()
            .await
            .expect("list_training_sessions failed");
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, second.id);
        assert_eq!(sessions[1].id, first.id);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_get_training_session_by_id() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");

        let created = store
            .create_training_session(NewTrainingSession {
                title: "Sessione".into(),
                created_by: None,
            })
            .await
            .expect("create failed");

        let fetched = store
            .get_training_session(created.id)
            .await
            .expect("get_training_session failed")
            .expect("should find the session");
        assert_eq!(fetched.id, created.id);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_none_when_getting_unknown_training_session() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");

        let result = store
            .get_training_session(999)
            .await
            .expect("get_training_session failed");
        assert!(result.is_none());
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_close_open_training_session_and_return_true() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");

        let created = store
            .create_training_session(NewTrainingSession {
                title: "Sessione".into(),
                created_by: None,
            })
            .await
            .expect("create failed");

        let closed = store
            .close_training_session(created.id)
            .await
            .expect("close_training_session failed");
        assert!(closed);

        let fetched = store
            .get_training_session(created.id)
            .await
            .expect("get failed")
            .expect("should find the session");
        assert!(fetched.closed_at.is_some());
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_false_when_closing_already_closed_training_session() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");

        let created = store
            .create_training_session(NewTrainingSession {
                title: "Sessione".into(),
                created_by: None,
            })
            .await
            .expect("create failed");
        store
            .close_training_session(created.id)
            .await
            .expect("first close failed");

        let second_close = store
            .close_training_session(created.id)
            .await
            .expect("close_training_session failed");
        assert!(!second_close);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_false_when_closing_unknown_training_session() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");

        let result = store
            .close_training_session(999)
            .await
            .expect("close_training_session failed");
        assert!(!result);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_create_training_message_with_all_fields() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let session = store
            .create_training_session(NewTrainingSession {
                title: "Sessione".into(),
                created_by: None,
            })
            .await
            .expect("create_training_session failed");

        let message = store
            .create_training_message(NewTrainingMessage {
                session_id: session.id,
                question: "A che ora apre l'anagrafe?".into(),
                answer: "Lo sportello apre alle 9:00.".into(),
                sources: r#"[{"document_id":1,"source_ref":"orari.md"}]"#.into(),
                fell_back: false,
            })
            .await
            .expect("create_training_message failed");

        assert!(message.id > 0);
        assert_eq!(message.session_id, session.id);
        assert_eq!(message.question, "A che ora apre l'anagrafe?");
        assert_eq!(message.answer, "Lo sportello apre alle 9:00.");
        assert_eq!(
            message.sources,
            r#"[{"document_id":1,"source_ref":"orari.md"}]"#
        );
        assert!(!message.fell_back);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_fail_to_create_training_message_for_nonexistent_session() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");

        let result = store
            .create_training_message(NewTrainingMessage {
                session_id: 999,
                question: "test".into(),
                answer: "test".into(),
                sources: "[]".into(),
                fell_back: true,
            })
            .await;

        assert!(
            result.is_err(),
            "the training_message.session_id foreign key should reject an unknown session"
        );
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_list_training_messages_oldest_first_for_session() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let session = store
            .create_training_session(NewTrainingSession {
                title: "Sessione".into(),
                created_by: None,
            })
            .await
            .expect("create_training_session failed");

        let first = store
            .create_training_message(NewTrainingMessage {
                session_id: session.id,
                question: "prima domanda".into(),
                answer: "prima risposta".into(),
                sources: "[]".into(),
                fell_back: false,
            })
            .await
            .expect("create failed");
        let second = store
            .create_training_message(NewTrainingMessage {
                session_id: session.id,
                question: "seconda domanda".into(),
                answer: "seconda risposta".into(),
                sources: "[]".into(),
                fell_back: false,
            })
            .await
            .expect("create failed");

        let messages = store
            .list_training_messages(session.id)
            .await
            .expect("list_training_messages failed");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id, first.id);
        assert_eq!(messages[1].id, second.id);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_empty_vec_when_listing_messages_for_session_with_none() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let session = store
            .create_training_session(NewTrainingSession {
                title: "Sessione".into(),
                created_by: None,
            })
            .await
            .expect("create_training_session failed");

        let messages = store
            .list_training_messages(session.id)
            .await
            .expect("list_training_messages failed");
        assert!(messages.is_empty());
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_only_list_training_messages_for_requested_session() {
        let path = temp_db_path();
        let store = KbStore::open(&path).await.expect("failed to open db");
        let session_a = store
            .create_training_session(NewTrainingSession {
                title: "Sessione A".into(),
                created_by: None,
            })
            .await
            .expect("create_training_session failed");
        let session_b = store
            .create_training_session(NewTrainingSession {
                title: "Sessione B".into(),
                created_by: None,
            })
            .await
            .expect("create_training_session failed");

        store
            .create_training_message(NewTrainingMessage {
                session_id: session_a.id,
                question: "domanda A".into(),
                answer: "risposta A".into(),
                sources: "[]".into(),
                fell_back: false,
            })
            .await
            .expect("create failed");
        store
            .create_training_message(NewTrainingMessage {
                session_id: session_b.id,
                question: "domanda B".into(),
                answer: "risposta B".into(),
                sources: "[]".into(),
                fell_back: false,
            })
            .await
            .expect("create failed");

        let messages_a = store
            .list_training_messages(session_a.id)
            .await
            .expect("list_training_messages failed");
        assert_eq!(messages_a.len(), 1);
        assert_eq!(messages_a[0].question, "domanda A");
        drop(store);
        let _ = std::fs::remove_file(&path);
    }
}
