use std::sync::Arc;

use async_trait::async_trait;
use kb_store::KbStore;

use crate::rag_engine::ports::RetrievalPort;
use crate::rag_engine::types::{RagError, RetrievedChunk};

pub struct RetrievalAdapter {
    store: Arc<KbStore>,
}

impl RetrievalAdapter {
    pub fn new(store: Arc<KbStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl RetrievalPort for RetrievalAdapter {
    async fn retrieve(
        &self,
        query_embedding: &[f32],
        top_k: i64,
        min_score: f64,
    ) -> Result<Vec<RetrievedChunk>, RagError> {
        let scored = self
            .store
            .search_similar(query_embedding, top_k, min_score)
            .await
            .map_err(|e| RagError::Retrieval(e.to_string()))?;

        Ok(scored
            .into_iter()
            .map(|s| RetrievedChunk {
                id: s.document.id,
                source_ref: s.document.source_ref,
                content: s.document.content,
                similarity: s.similarity,
                source_url: extract_source_url(s.document.metadata.as_deref()),
            })
            .collect())
    }
}

/// Best-effort: a missing or malformed `metadata` blob (any document
/// ingested before this field existed, or a manual upload with no known
/// URL) must never break retrieval — it just means no link is offered.
fn extract_source_url(metadata: Option<&str>) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(metadata?).ok()?;
    value.get("source_url")?.as_str().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kb_store::{DocumentSource, EMBEDDING_DIM, NewDocument};
    use std::sync::atomic::{AtomicU32, Ordering};

    static DB_COUNTER: AtomicU32 = AtomicU32::new(1);

    fn temp_db_path() -> String {
        let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir()
            .join(format!("retrieval_adapter_test_{n}.db"))
            .to_string_lossy()
            .into_owned()
    }

    #[tokio::test]
    async fn should_return_nearest_document_first() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());

        let vec_a: Vec<f32> = (0..EMBEDDING_DIM).map(|i| i as f32 * 0.001).collect();
        let vec_b: Vec<f32> = (0..EMBEDDING_DIM)
            .map(|i| i as f32 * 0.001 + 10.0)
            .collect();

        store
            .insert_document(NewDocument {
                source: DocumentSource::Manual,
                source_ref: "doc_a".into(),
                content: "content_a".into(),
                metadata: None,
                embedding: vec_a.clone(),
                section: None,
            })
            .await
            .unwrap();

        store
            .insert_document(NewDocument {
                source: DocumentSource::Manual,
                source_ref: "doc_b".into(),
                content: "content_b".into(),
                metadata: None,
                embedding: vec_b.clone(),
                section: None,
            })
            .await
            .unwrap();

        let adapter = RetrievalAdapter::new(store.clone());
        let results = adapter.retrieve(&vec_a, 5, -1.0).await.unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].source_ref, "doc_a");
        assert!(results[0].similarity > results[1].similarity);

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_empty_when_no_chunks_above_threshold() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());

        let embedding = vec![0.1f32; EMBEDDING_DIM];
        store
            .insert_document(NewDocument {
                source: DocumentSource::Manual,
                source_ref: "doc".into(),
                content: "content".into(),
                metadata: None,
                embedding,
                section: None,
            })
            .await
            .unwrap();

        let adapter = RetrievalAdapter::new(store.clone());
        let results = adapter
            .retrieve(&[0.1f32; EMBEDDING_DIM], 5, 2.0)
            .await
            .unwrap();

        assert!(results.is_empty());

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_map_scored_document_to_retrieved_chunk() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());

        let embedding = vec![0.1f32; EMBEDDING_DIM];
        store
            .insert_document(NewDocument {
                source: DocumentSource::Manual,
                source_ref: "anagrafe.md".into(),
                content: "Orari: 9-12:30".into(),
                metadata: None,
                embedding,
                section: None,
            })
            .await
            .unwrap();

        let adapter = RetrievalAdapter::new(store.clone());
        let results = adapter
            .retrieve(&[0.1f32; EMBEDDING_DIM], 5, -1.0)
            .await
            .unwrap();

        let chunk = &results[0];
        assert_eq!(chunk.source_ref, "anagrafe.md");
        assert_eq!(chunk.content, "Orari: 9-12:30");
        assert!(chunk.id > 0);
        assert_eq!(chunk.source_url, None);

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_extract_source_url_from_metadata_when_present() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());

        let embedding = vec![0.1f32; EMBEDDING_DIM];
        store
            .insert_document(NewDocument {
                source: DocumentSource::Scrape,
                source_ref: "delibera-di-giunta-74-2026-07-13.pdf".into(),
                content: "Contenuto della delibera 74.".into(),
                metadata: Some(r#"{"source_url": "https://www.halleyweb.com/detail/74"}"#.into()),
                embedding,
                section: Some("delibere".into()),
            })
            .await
            .unwrap();

        let adapter = RetrievalAdapter::new(store.clone());
        let results = adapter
            .retrieve(&[0.1f32; EMBEDDING_DIM], 5, -1.0)
            .await
            .unwrap();

        assert_eq!(
            results[0].source_url.as_deref(),
            Some("https://www.halleyweb.com/detail/74")
        );

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_tolerate_malformed_metadata_without_failing_retrieval() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());

        let embedding = vec![0.1f32; EMBEDDING_DIM];
        store
            .insert_document(NewDocument {
                source: DocumentSource::Manual,
                source_ref: "old-upload.md".into(),
                content: "Contenuto.".into(),
                metadata: Some("not valid json".into()),
                embedding,
                section: None,
            })
            .await
            .unwrap();

        let adapter = RetrievalAdapter::new(store.clone());
        let results = adapter
            .retrieve(&[0.1f32; EMBEDDING_DIM], 5, -1.0)
            .await
            .unwrap();

        assert_eq!(results[0].source_url, None);

        drop(store);
        let _ = std::fs::remove_file(&path);
    }
}
