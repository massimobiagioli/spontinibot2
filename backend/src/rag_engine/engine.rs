use std::sync::Arc;

use crate::rag_engine::identity::is_identity_question;
use crate::rag_engine::ports::{
    EmbeddingPort, GenerationPort, PersonaPort, RetrievalPort, TrainingNotesPort,
};
use crate::rag_engine::prompt;
use crate::rag_engine::training_notes::NoopTrainingNotes;
use crate::rag_engine::types::{Answer, CitedSource, RagError};

pub struct RagEngine {
    embedding: Arc<dyn EmbeddingPort>,
    retrieval: Arc<dyn RetrievalPort>,
    persona: Arc<dyn PersonaPort>,
    generation: Arc<dyn GenerationPort>,
    training_notes: Arc<dyn TrainingNotesPort>,
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
            training_notes: Arc::new(NoopTrainingNotes),
            top_k,
            min_score,
        }
    }

    /// Opts a real `TrainingNotesPort` in (ADR 0016). Every existing
    /// `RagEngine::new()` call site — including the whole test suite —
    /// keeps working unchanged, defaulting to no training notes.
    pub fn with_training_notes(mut self, training_notes: Arc<dyn TrainingNotesPort>) -> Self {
        self.training_notes = training_notes;
        self
    }

    pub async fn answer(&self, question: &str) -> Result<Answer, RagError> {
        let persona = self
            .persona
            .active_persona()
            .await?
            .ok_or(RagError::NoActivePersona)?;

        // ADR 0014: identity/imprinting questions ("Chi sei?") are answered
        // directly from the persona's own configuration — never via
        // embedding/retrieval/generation. The answer already exists; a full
        // RAG round trip cannot be "ultra-immediate" under any tuning.
        if is_identity_question(question, &persona.name) {
            return Ok(Answer {
                text: persona.system_prompt,
                sources: vec![],
                fell_back: false,
            });
        }

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

        // Best-effort: an unreadable/misconfigured training-notes directory
        // must never break answering — it's supplementary, not load-bearing.
        let notes = self
            .training_notes
            .training_notes()
            .await
            .unwrap_or_default();
        let prompt = prompt::assemble(&persona, &chunks, question, &notes);
        let text = self.generation.generate(prompt).await?;
        let sources = chunks
            .iter()
            .map(|c| CitedSource {
                document_id: c.id,
                source_ref: c.source_ref.clone(),
                source_url: c.source_url.clone(),
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

    struct CountingEmbedding {
        call_count: AtomicUsize,
    }

    #[async_trait]
    impl EmbeddingPort for CountingEmbedding {
        async fn embed(&self, _text: &str) -> Result<Vec<f32>, RagError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(vec![0.1; 768])
        }
    }

    struct CountingRetrieval {
        call_count: AtomicUsize,
    }

    #[async_trait]
    impl RetrievalPort for CountingRetrieval {
        async fn retrieve(
            &self,
            _qe: &[f32],
            _top_k: i64,
            _min_score: f64,
        ) -> Result<Vec<RetrievedChunk>, RagError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(vec![])
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

        async fn reload_persona(&self) -> Result<(), RagError> {
            Ok(())
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

    struct CapturingGeneration {
        received: std::sync::Mutex<Option<PromptParts>>,
    }

    #[async_trait]
    impl GenerationPort for CapturingGeneration {
        async fn generate(&self, prompt: PromptParts) -> Result<String, RagError> {
            *self.received.lock().unwrap() = Some(prompt);
            Ok("Test answer".into())
        }
    }

    struct TestTrainingNotes {
        notes: String,
    }

    #[async_trait]
    impl TrainingNotesPort for TestTrainingNotes {
        async fn training_notes(&self) -> Result<String, RagError> {
            Ok(self.notes.clone())
        }
    }

    fn sample_persona() -> crate::rag_engine::types::PersonaSnapshot {
        crate::rag_engine::types::PersonaSnapshot {
            name: "gaspare".into(),
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
            source_url: None,
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
    async fn should_carry_chunk_source_url_through_to_cited_source() {
        let mut chunks = sample_chunks();
        chunks[0].source_url = Some("https://www.halleyweb.com/detail/74".into());

        let engine = RagEngine::new(
            Arc::new(TestEmbedding),
            Arc::new(TestRetrieval { chunks }),
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

        assert_eq!(
            answer.sources[0].source_url.as_deref(),
            Some("https://www.halleyweb.com/detail/74")
        );
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

    #[tokio::test]
    async fn should_answer_identity_question_directly_from_persona_without_calling_any_port() {
        let embed_counter = Arc::new(CountingEmbedding {
            call_count: AtomicUsize::new(0),
        });
        let retrieval_counter = Arc::new(CountingRetrieval {
            call_count: AtomicUsize::new(0),
        });
        let gen_counter = Arc::new(TestGeneration {
            call_count: AtomicUsize::new(0),
        });

        let engine = RagEngine::new(
            embed_counter.clone(),
            retrieval_counter.clone(),
            Arc::new(TestPersona {
                snapshot: Some(sample_persona()),
            }),
            gen_counter.clone(),
            5,
            0.35,
        );

        let answer = engine.answer("Chi sei?").await.unwrap();

        assert_eq!(answer.text, "Sei Gaspare Spontini.");
        assert!(answer.sources.is_empty());
        assert!(!answer.fell_back);
        assert_eq!(embed_counter.call_count.load(Ordering::SeqCst), 0);
        assert_eq!(retrieval_counter.call_count.load(Ordering::SeqCst), 0);
        assert_eq!(gen_counter.call_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn should_answer_who_is_persona_name_question_directly_from_persona() {
        // The question is derived from the mock persona's own `name` field
        // (a real, operator-editable imprinting value) rather than a
        // separately hardcoded literal, so this test can't silently drift
        // out of sync if `sample_persona()` is ever renamed.
        let persona = sample_persona();
        let question = format!("Chi è {}?", persona.name);

        let engine = RagEngine::new(
            Arc::new(TestEmbedding),
            Arc::new(TestRetrieval { chunks: vec![] }),
            Arc::new(TestPersona {
                snapshot: Some(persona.clone()),
            }),
            Arc::new(TestGeneration {
                call_count: AtomicUsize::new(0),
            }),
            5,
            0.35,
        );

        let answer = engine.answer(&question).await.unwrap();

        assert_eq!(answer.text, persona.system_prompt);
        assert!(!answer.fell_back);
    }

    #[tokio::test]
    async fn should_append_training_notes_to_system_prompt_when_configured() {
        let generation = Arc::new(CapturingGeneration {
            received: std::sync::Mutex::new(None),
        });

        let engine = RagEngine::new(
            Arc::new(TestEmbedding),
            Arc::new(TestRetrieval {
                chunks: sample_chunks(),
            }),
            Arc::new(TestPersona {
                snapshot: Some(sample_persona()),
            }),
            generation.clone(),
            5,
            0.35,
        )
        .with_training_notes(Arc::new(TestTrainingNotes {
            notes: "Non citare mai una fonte che non fonda davvero la risposta.".into(),
        }));

        engine.answer("A che ora apre l'anagrafe?").await.unwrap();

        let received = generation.received.lock().unwrap();
        let prompt = received
            .as_ref()
            .expect("generate() should have been called");
        assert!(prompt.system.starts_with("Sei Gaspare Spontini."));
        assert!(
            prompt
                .system
                .contains("Non citare mai una fonte che non fonda davvero la risposta.")
        );
        assert!(!prompt.context.contains("Non citare mai"));
        assert!(!prompt.user.contains("Non citare mai"));
    }

    #[tokio::test]
    async fn should_default_to_empty_training_notes_when_not_configured() {
        let generation = Arc::new(CapturingGeneration {
            received: std::sync::Mutex::new(None),
        });

        let engine = RagEngine::new(
            Arc::new(TestEmbedding),
            Arc::new(TestRetrieval {
                chunks: sample_chunks(),
            }),
            Arc::new(TestPersona {
                snapshot: Some(sample_persona()),
            }),
            generation.clone(),
            5,
            0.35,
        );

        engine.answer("A che ora apre l'anagrafe?").await.unwrap();

        let received = generation.received.lock().unwrap();
        let prompt = received
            .as_ref()
            .expect("generate() should have been called");
        assert_eq!(prompt.system, "Sei Gaspare Spontini.");
    }
}
