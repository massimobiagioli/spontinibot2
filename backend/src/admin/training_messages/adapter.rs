use std::sync::Arc;

use async_trait::async_trait;

use kb_store::{KbStore, NewTrainingMessage};

use super::{
    AskTrainingMessageRequest, TrainingMessageAdminPort, TrainingMessageError,
    TrainingMessageResponse, TrainingMessageSource,
};
use crate::rag_engine::engine::RagEngine;
use crate::rag_engine::types::RagError;

pub struct RagTrainingMessageAdapter {
    store: Arc<KbStore>,
    rag_engine: Arc<RagEngine>,
}

impl RagTrainingMessageAdapter {
    pub fn new(store: Arc<KbStore>, rag_engine: Arc<RagEngine>) -> Self {
        Self { store, rag_engine }
    }
}

#[async_trait]
impl TrainingMessageAdminPort for RagTrainingMessageAdapter {
    async fn ask(
        &self,
        session_id: i64,
        req: AskTrainingMessageRequest,
    ) -> Result<TrainingMessageResponse, TrainingMessageError> {
        self.store
            .get_training_session(session_id)
            .await?
            .ok_or(TrainingMessageError::SessionNotFound(session_id))?;

        let AskTrainingMessageRequest {
            question,
            expected_answer,
            answer,
        } = req;

        let new_message = if let Some(manual_answer) = answer {
            NewTrainingMessage {
                session_id,
                question,
                answer: manual_answer,
                sources: "[]".into(),
                fell_back: false,
                expected_answer,
                execution_time_ms: None,
                source: "manual".into(),
            }
        } else {
            let started_at = std::time::Instant::now();
            let answer = self
                .rag_engine
                .answer(&question)
                .await
                .map_err(|e: RagError| TrainingMessageError::Rag(e.to_string()))?;
            let execution_time_ms = started_at.elapsed().as_millis() as i64;

            let sources: Vec<TrainingMessageSource> = answer
                .sources
                .iter()
                .map(|s| TrainingMessageSource {
                    document_id: s.document_id,
                    source_ref: s.source_ref.clone(),
                })
                .collect();
            let sources_json = serde_json::to_string(&sources)
                .expect("Vec<TrainingMessageSource> of plain fields is always serializable");

            NewTrainingMessage {
                session_id,
                question,
                answer: answer.text,
                sources: sources_json,
                fell_back: answer.fell_back,
                expected_answer,
                execution_time_ms: Some(execution_time_ms),
                source: "chat".into(),
            }
        };

        let message = self.store.create_training_message(new_message).await?;

        message_to_response(message)
    }

    async fn list_messages(
        &self,
        session_id: i64,
    ) -> Result<Vec<TrainingMessageResponse>, TrainingMessageError> {
        let messages = self.store.list_training_messages(session_id).await?;
        messages.into_iter().map(message_to_response).collect()
    }

    async fn update_expected_answer(
        &self,
        message_id: i64,
        expected_answer: Option<String>,
    ) -> Result<TrainingMessageResponse, TrainingMessageError> {
        let message = self
            .store
            .update_training_message_expected_answer(message_id, expected_answer)
            .await?
            .ok_or(TrainingMessageError::MessageNotFound(message_id))?;
        message_to_response(message)
    }
}

fn message_to_response(
    message: kb_store::TrainingMessage,
) -> Result<TrainingMessageResponse, TrainingMessageError> {
    let sources = serde_json::from_str(&message.sources)
        .map_err(|e| TrainingMessageError::Serialization(e.to_string()))?;
    Ok(TrainingMessageResponse {
        id: message.id,
        session_id: message.session_id,
        question: message.question,
        answer: message.answer,
        sources,
        fell_back: message.fell_back,
        created_at: message.created_at,
        expected_answer: message.expected_answer,
        execution_time_ms: message.execution_time_ms,
        source: message.source,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    use async_trait::async_trait;
    use kb_store::KbStore;

    use crate::admin::training_messages::adapter::RagTrainingMessageAdapter;
    use crate::admin::training_messages::{
        AskTrainingMessageRequest, TrainingMessageAdminPort, TrainingMessageError,
    };
    use crate::rag_engine::engine::RagEngine;
    use crate::rag_engine::ports::{EmbeddingPort, GenerationPort, PersonaPort, RetrievalPort};
    use crate::rag_engine::types::{PersonaSnapshot, PromptParts, RagError, RetrievedChunk};

    static DB_COUNTER: AtomicU32 = AtomicU32::new(1);

    async fn temp_store() -> KbStore {
        let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir();
        let path = dir.join(format!("training_message_adapter_test_{n}.db"));
        let _ = std::fs::remove_file(&path);
        KbStore::open(&path.to_string_lossy())
            .await
            .expect("failed to open temp db")
    }

    struct TestEmbedding;

    #[async_trait]
    impl EmbeddingPort for TestEmbedding {
        async fn embed(&self, _text: &str) -> Result<Vec<f32>, RagError> {
            Ok(vec![0.1; 768])
        }
    }

    struct FailingEmbedding;

    #[async_trait]
    impl EmbeddingPort for FailingEmbedding {
        async fn embed(&self, _text: &str) -> Result<Vec<f32>, RagError> {
            Err(RagError::Embedding("embedding service unavailable".into()))
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
        snapshot: Option<PersonaSnapshot>,
    }

    #[async_trait]
    impl PersonaPort for TestPersona {
        async fn active_persona(&self) -> Result<Option<PersonaSnapshot>, RagError> {
            Ok(self.snapshot.clone())
        }

        async fn reload_persona(&self) -> Result<(), RagError> {
            Ok(())
        }
    }

    struct TestGeneration;

    #[async_trait]
    impl GenerationPort for TestGeneration {
        async fn generate(&self, _prompt: PromptParts) -> Result<String, RagError> {
            Ok("Lo sportello apre alle 9:00.".into())
        }
    }

    fn sample_persona() -> PersonaSnapshot {
        PersonaSnapshot {
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
        }]
    }

    fn engine_with_chunks(chunks: Vec<RetrievedChunk>) -> Arc<RagEngine> {
        Arc::new(RagEngine::new(
            Arc::new(TestEmbedding),
            Arc::new(TestRetrieval { chunks }),
            Arc::new(TestPersona {
                snapshot: Some(sample_persona()),
            }),
            Arc::new(TestGeneration),
            5,
            0.35,
        ))
    }

    fn ask_req(question: &str) -> AskTrainingMessageRequest {
        AskTrainingMessageRequest {
            question: question.into(),
            expected_answer: None,
            answer: None,
        }
    }

    #[test]
    fn should_map_malformed_sources_json_to_serialization_error() {
        let message = kb_store::TrainingMessage {
            id: 1,
            session_id: 1,
            question: "domanda".into(),
            answer: "risposta".into(),
            sources: "not valid json".into(),
            fell_back: false,
            created_at: "2026-07-24T00:00:00Z".into(),
            expected_answer: None,
            execution_time_ms: Some(10),
            source: "chat".into(),
        };

        let result = super::message_to_response(message);
        assert!(matches!(
            result,
            Err(TrainingMessageError::Serialization(_))
        ));
    }

    #[tokio::test]
    async fn should_map_rag_engine_error_to_training_message_rag_error() {
        let store = Arc::new(temp_store().await);
        let session = store
            .create_training_session(kb_store::NewTrainingSession {
                title: "Sessione".into(),
                created_by: None,
            })
            .await
            .expect("create_training_session failed");
        let rag_engine = Arc::new(RagEngine::new(
            Arc::new(FailingEmbedding),
            Arc::new(TestRetrieval {
                chunks: sample_chunks(),
            }),
            Arc::new(TestPersona {
                snapshot: Some(sample_persona()),
            }),
            Arc::new(TestGeneration),
            5,
            0.35,
        ));
        let adapter = RagTrainingMessageAdapter::new(store, rag_engine);

        let result = adapter.ask(session.id, ask_req("domanda")).await;
        assert!(matches!(result, Err(TrainingMessageError::Rag(_))));
    }

    #[tokio::test]
    async fn should_return_session_not_found_for_unknown_session() {
        let store = Arc::new(temp_store().await);
        let rag_engine = engine_with_chunks(sample_chunks());
        let adapter = RagTrainingMessageAdapter::new(store, rag_engine);

        let result = adapter.ask(999, ask_req("domanda")).await;
        assert!(matches!(
            result,
            Err(TrainingMessageError::SessionNotFound(999))
        ));
    }

    #[tokio::test]
    async fn should_ask_and_persist_message_for_known_session() {
        let store = Arc::new(temp_store().await);
        let session = store
            .create_training_session(kb_store::NewTrainingSession {
                title: "Sessione".into(),
                created_by: None,
            })
            .await
            .expect("create_training_session failed");
        let rag_engine = engine_with_chunks(sample_chunks());
        let adapter = RagTrainingMessageAdapter::new(store, rag_engine);

        let response = adapter
            .ask(session.id, ask_req("A che ora apre l'anagrafe?"))
            .await
            .expect("ask failed");

        assert_eq!(response.session_id, session.id);
        assert_eq!(response.question, "A che ora apre l'anagrafe?");
        assert_eq!(response.answer, "Lo sportello apre alle 9:00.");
        assert!(!response.fell_back);
        assert_eq!(response.source, "chat");
        assert!(response.execution_time_ms.is_some());
        assert_eq!(response.sources.len(), 1);
        assert_eq!(response.sources[0].source_ref, "orari.md");
    }

    #[tokio::test]
    async fn should_update_expected_answer_on_existing_message() {
        let store = Arc::new(temp_store().await);
        let session = store
            .create_training_session(kb_store::NewTrainingSession {
                title: "Sessione".into(),
                created_by: None,
            })
            .await
            .expect("create_training_session failed");
        let rag_engine = engine_with_chunks(sample_chunks());
        let adapter = RagTrainingMessageAdapter::new(store, rag_engine);
        let created = adapter
            .ask(session.id, ask_req("A che ora apre l'anagrafe?"))
            .await
            .expect("ask failed");
        assert_eq!(created.expected_answer, None);

        let updated = adapter
            .update_expected_answer(created.id, Some("Dalle 9:00 alle 12:30".into()))
            .await
            .expect("update_expected_answer failed");

        assert_eq!(updated.id, created.id);
        assert_eq!(
            updated.expected_answer.as_deref(),
            Some("Dalle 9:00 alle 12:30")
        );
        // Everything else about the message is untouched by the update.
        assert_eq!(updated.answer, created.answer);
        assert_eq!(updated.question, created.question);
    }

    #[tokio::test]
    async fn should_return_message_not_found_when_updating_unknown_message() {
        let store = Arc::new(temp_store().await);
        let rag_engine = engine_with_chunks(sample_chunks());
        let adapter = RagTrainingMessageAdapter::new(store, rag_engine);

        let result = adapter
            .update_expected_answer(999, Some("x".into()))
            .await;

        assert!(matches!(
            result,
            Err(TrainingMessageError::MessageNotFound(999))
        ));
    }

    #[tokio::test]
    async fn should_record_manual_answer_without_invoking_rag_engine() {
        let store = Arc::new(temp_store().await);
        let session = store
            .create_training_session(kb_store::NewTrainingSession {
                title: "Sessione".into(),
                created_by: None,
            })
            .await
            .expect("create_training_session failed");
        // FailingEmbedding would error out any live RAG call, proving the
        // manual path never invokes the rag engine.
        let rag_engine = Arc::new(RagEngine::new(
            Arc::new(FailingEmbedding),
            Arc::new(TestRetrieval {
                chunks: sample_chunks(),
            }),
            Arc::new(TestPersona {
                snapshot: Some(sample_persona()),
            }),
            Arc::new(TestGeneration),
            5,
            0.35,
        ));
        let adapter = RagTrainingMessageAdapter::new(store, rag_engine);

        let response = adapter
            .ask(
                session.id,
                AskTrainingMessageRequest {
                    question: "A che ora apre l'anagrafe?".into(),
                    expected_answer: Some("Dalle 9:00 alle 12:30".into()),
                    answer: Some("Lo sportello apre alle 9:00.".into()),
                },
            )
            .await
            .expect("ask failed");

        assert_eq!(response.answer, "Lo sportello apre alle 9:00.");
        assert_eq!(
            response.expected_answer.as_deref(),
            Some("Dalle 9:00 alle 12:30")
        );
        assert_eq!(response.source, "manual");
        assert!(response.execution_time_ms.is_none());
        assert!(response.sources.is_empty());
        assert!(!response.fell_back);
    }

    #[tokio::test]
    async fn should_record_honest_unknown_fallback_with_no_sources() {
        let store = Arc::new(temp_store().await);
        let session = store
            .create_training_session(kb_store::NewTrainingSession {
                title: "Sessione".into(),
                created_by: None,
            })
            .await
            .expect("create_training_session failed");
        let rag_engine = engine_with_chunks(vec![]);
        let adapter = RagTrainingMessageAdapter::new(store, rag_engine);

        let response = adapter
            .ask(session.id, ask_req("domanda senza risposta"))
            .await
            .expect("ask failed");

        assert!(response.fell_back);
        assert!(response.sources.is_empty());
    }

    #[tokio::test]
    async fn should_list_messages_oldest_first() {
        let store = Arc::new(temp_store().await);
        let session = store
            .create_training_session(kb_store::NewTrainingSession {
                title: "Sessione".into(),
                created_by: None,
            })
            .await
            .expect("create_training_session failed");
        let rag_engine = engine_with_chunks(sample_chunks());
        let adapter = RagTrainingMessageAdapter::new(store, rag_engine);

        adapter
            .ask(session.id, ask_req("prima domanda"))
            .await
            .expect("ask failed");
        adapter
            .ask(session.id, ask_req("seconda domanda"))
            .await
            .expect("ask failed");

        let messages = adapter
            .list_messages(session.id)
            .await
            .expect("list_messages failed");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].question, "prima domanda");
        assert_eq!(messages[1].question, "seconda domanda");
    }
}
