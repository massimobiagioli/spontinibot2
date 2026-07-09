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
            })
            .collect())
    }
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

        drop(store);
        let _ = std::fs::remove_file(&path);
    }
}
