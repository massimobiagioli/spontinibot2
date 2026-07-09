use std::sync::Arc;

use crate::rag_engine::ports::{EmbeddingPort, GenerationPort, PersonaPort, RetrievalPort};
use crate::rag_engine::prompt;
use crate::rag_engine::types::{Answer, CitedSource, RagError};

pub struct RagEngine {
    embedding: Arc<dyn EmbeddingPort>,
    retrieval: Arc<dyn RetrievalPort>,
    persona: Arc<dyn PersonaPort>,
    generation: Arc<dyn GenerationPort>,
    top_k: i64,
    min_score: f64,
}

impl RagEngine {
    pub fn new(
        embedding: Arc<dyn EmbeddingPort>,
        retrieval: Arc<dyn RetrievalPort>,
        persona: Arc<dyn PersonaPort>,
        generation: Arc<dyn GenerationPort>,
        top_k: i64,
        min_score: f64,
    ) -> Self {
        Self {
            embedding,
            retrieval,
            persona,
            generation,
            top_k,
            min_score,
        }
    }

    pub async fn answer(&self, question: &str) -> Result<Answer, RagError> {
        let persona = self
            .persona
            .active_persona()
            .await?
            .ok_or(RagError::NoActivePersona)?;

        let qe = self.embedding.embed(question).await?;
        let chunks = self
            .retrieval
            .retrieve(&qe, self.top_k, self.min_score)
            .await?;

        if chunks.is_empty() {
            let fallback = persona.fallback_message.unwrap_or_else(|| {
                "Non ho trovato informazioni nei documenti comunali su questo argomento.".into()
            });
            return Ok(Answer {
                text: fallback,
                sources: vec![],
                fell_back: true,
            });
        }

        let prompt = prompt::assemble(&persona, &chunks, question);
        let text = self.generation.generate(prompt).await?;
        let sources = chunks
            .iter()
            .map(|c| CitedSource {
                document_id: c.id,
                source_ref: c.source_ref.clone(),
            })
            .collect();

        Ok(Answer {
            text,
            sources,
            fell_back: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag_engine::ports::RetrievalPort;
    use crate::rag_engine::types::{PromptParts, RetrievedChunk};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestEmbedding;

    #[async_trait]
    impl EmbeddingPort for TestEmbedding {
        async fn embed(&self, _text: &str) -> Result<Vec<f32>, RagError> {
            Ok(vec![0.1; 768])
        }
    }

    struct TestEmbeddingError;

    #[async_trait]
    impl EmbeddingPort for TestEmbeddingError {
        async fn embed(&self, _text: &str) -> Result<Vec<f32>, RagError> {
            Err(RagError::Embedding("mock error".into()))
        }
    }

    struct TestRetrieval {
        chunks: Vec<RetrievedChunk>,
    }

    #[async_trait]
    impl RetrievalPort for TestRetrieval {
        async fn retrieve(
            &self,
            _qe: &[f32],
            _top_k: i64,
            _min_score: f64,
        ) -> Result<Vec<RetrievedChunk>, RagError> {
            Ok(self.chunks.clone())
        }
    }

    struct TestPersona {
        snapshot: Option<crate::rag_engine::types::PersonaSnapshot>,
    }

    #[async_trait]
    impl PersonaPort for TestPersona {
        async fn active_persona(
            &self,
        ) -> Result<Option<crate::rag_engine::types::PersonaSnapshot>, RagError> {
            Ok(self.snapshot.clone())
        }
    }

    struct TestGeneration {
        call_count: AtomicUsize,
    }

    #[async_trait]
    impl GenerationPort for TestGeneration {
        async fn generate(&self, _prompt: PromptParts) -> Result<String, RagError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok("Test answer".into())
        }
    }

    fn sample_persona() -> crate::rag_engine::types::PersonaSnapshot {
        crate::rag_engine::types::PersonaSnapshot {
            system_prompt: "Sei Gaspare Spontini.".into(),
            fallback_message: None,
        }
    }

    fn sample_chunks() -> Vec<RetrievedChunk> {
        vec![RetrievedChunk {
            id: 1,
            content: "L'anagrafe apre alle 9:00.".into(),
            source_ref: "orari.md".into(),
            similarity: 0.85,
        }]
    }

    #[tokio::test]
    async fn should_return_grounded_answer_with_cited_sources_when_chunks_found() {
        let engine = RagEngine::new(
            Arc::new(TestEmbedding),
            Arc::new(TestRetrieval {
                chunks: sample_chunks(),
            }),
            Arc::new(TestPersona {
                snapshot: Some(sample_persona()),
            }),
            Arc::new(TestGeneration {
                call_count: AtomicUsize::new(0),
            }),
            5,
            0.35,
        );

        let answer = engine.answer("A che ora apre l'anagrafe?").await.unwrap();

        assert_eq!(answer.text, "Test answer");
        assert!(!answer.fell_back);
        assert_eq!(answer.sources.len(), 1);
        assert_eq!(answer.sources[0].source_ref, "orari.md");
    }

    #[tokio::test]
    async fn should_return_fallback_answer_with_no_sources_when_no_chunks() {
        let engine = RagEngine::new(
            Arc::new(TestEmbedding),
            Arc::new(TestRetrieval { chunks: vec![] }),
            Arc::new(TestPersona {
                snapshot: Some(sample_persona()),
            }),
            Arc::new(TestGeneration {
                call_count: AtomicUsize::new(0),
            }),
            5,
            0.35,
        );

        let answer = engine.answer("test question").await.unwrap();

        assert!(answer.fell_back);
        assert!(answer.sources.is_empty());
        assert!(answer.text.contains("Non ho trovato informazioni"));
    }

    #[tokio::test]
    async fn should_return_no_active_persona_error_when_persona_missing() {
        let engine = RagEngine::new(
            Arc::new(TestEmbedding),
            Arc::new(TestRetrieval { chunks: vec![] }),
            Arc::new(TestPersona { snapshot: None }),
            Arc::new(TestGeneration {
                call_count: AtomicUsize::new(0),
            }),
            5,
            0.35,
        );

        let err = engine.answer("test").await.unwrap_err();
        assert!(matches!(err, RagError::NoActivePersona));
    }

    #[tokio::test]
    async fn should_propagate_embedding_error() {
        let engine = RagEngine::new(
            Arc::new(TestEmbeddingError),
            Arc::new(TestRetrieval { chunks: vec![] }),
            Arc::new(TestPersona {
                snapshot: Some(sample_persona()),
            }),
            Arc::new(TestGeneration {
                call_count: AtomicUsize::new(0),
            }),
            5,
            0.35,
        );

        let err = engine.answer("test").await.unwrap_err();
        assert!(matches!(err, RagError::Embedding(_)));
    }

    #[tokio::test]
    async fn should_not_call_generation_when_no_chunks_found() {
        let gen_counter = Arc::new(TestGeneration {
            call_count: AtomicUsize::new(0),
        });

        let engine = RagEngine::new(
            Arc::new(TestEmbedding),
            Arc::new(TestRetrieval { chunks: vec![] }),
            Arc::new(TestPersona {
                snapshot: Some(sample_persona()),
            }),
            gen_counter.clone(),
            5,
            0.35,
        );

        let _ = engine.answer("test").await.unwrap();
        assert_eq!(gen_counter.call_count.load(Ordering::SeqCst), 0);
    }
}
