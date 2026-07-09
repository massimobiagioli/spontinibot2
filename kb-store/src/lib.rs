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
    Document, DocumentSource, EMBEDDING_DIM, NewDocument, NewPersona, Persona, ScoredDocument,
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
}
