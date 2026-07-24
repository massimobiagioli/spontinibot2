use std::sync::Arc;

use async_trait::async_trait;

use kb_store::{KbStore, NewTrainingFeedback};

use super::{TrainingFeedbackAdminPort, TrainingFeedbackError, TrainingFeedbackResponse};

pub struct KbStoreTrainingFeedbackAdapter {
    store: Arc<KbStore>,
}

impl KbStoreTrainingFeedbackAdapter {
    pub fn new(store: Arc<KbStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl TrainingFeedbackAdminPort for KbStoreTrainingFeedbackAdapter {
    async fn create_feedback(
        &self,
        req: NewTrainingFeedback,
    ) -> Result<TrainingFeedbackResponse, TrainingFeedbackError> {
        self.store
            .get_training_message(req.message_id)
            .await?
            .ok_or(TrainingFeedbackError::MessageNotFound(req.message_id))?;

        let feedback = self.store.create_training_feedback(req).await?;
        Ok(TrainingFeedbackResponse::from(feedback))
    }

    async fn list_feedback(
        &self,
        message_id: i64,
    ) -> Result<Vec<TrainingFeedbackResponse>, TrainingFeedbackError> {
        let feedback = self.store.list_training_feedback(message_id).await?;
        Ok(feedback
            .into_iter()
            .map(TrainingFeedbackResponse::from)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    use kb_store::{
        DocumentSource, EMBEDDING_DIM, KbStore, NewDocument, NewTrainingFeedback,
        NewTrainingMessage, NewTrainingSession, Sentiment,
    };

    use crate::admin::training_feedback::TrainingFeedbackAdminPort;
    use crate::admin::training_feedback::adapter::KbStoreTrainingFeedbackAdapter;

    static DB_COUNTER: AtomicU32 = AtomicU32::new(1);

    async fn temp_store() -> KbStore {
        let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir();
        let path = dir.join(format!("training_feedback_adapter_test_{n}.db"));
        let _ = std::fs::remove_file(&path);
        KbStore::open(&path.to_string_lossy())
            .await
            .expect("failed to open temp db")
    }

    async fn sample_message_id(store: &KbStore) -> i64 {
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
                question: "domanda".into(),
                answer: "risposta".into(),
                sources: "[]".into(),
                fell_back: false,
            })
            .await
            .expect("create_training_message failed");
        message.id
    }

    #[tokio::test]
    async fn should_return_db_error_for_nonexistent_chunk_id() {
        let store = Arc::new(temp_store().await);
        let message_id = sample_message_id(&store).await;
        let adapter = KbStoreTrainingFeedbackAdapter::new(store);

        let result = adapter
            .create_feedback(NewTrainingFeedback {
                message_id,
                chunk_id: Some(999),
                answer_span: "test".into(),
                sentiment: Sentiment::Positive,
                comment: None,
            })
            .await;

        assert!(matches!(
            result,
            Err(crate::admin::training_feedback::TrainingFeedbackError::DbError(_))
        ));
    }

    #[tokio::test]
    async fn should_return_message_not_found_for_unknown_message() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreTrainingFeedbackAdapter::new(store);

        let result = adapter
            .create_feedback(NewTrainingFeedback {
                message_id: 999,
                chunk_id: None,
                answer_span: "test".into(),
                sentiment: Sentiment::Positive,
                comment: None,
            })
            .await;

        assert!(matches!(
            result,
            Err(crate::admin::training_feedback::TrainingFeedbackError::MessageNotFound(999))
        ));
    }

    #[tokio::test]
    async fn should_create_feedback_for_known_message() {
        let store = Arc::new(temp_store().await);
        let message_id = sample_message_id(&store).await;
        let adapter = KbStoreTrainingFeedbackAdapter::new(store);

        let response = adapter
            .create_feedback(NewTrainingFeedback {
                message_id,
                chunk_id: None,
                answer_span: "alle 9:00".into(),
                sentiment: Sentiment::Positive,
                comment: None,
            })
            .await
            .expect("create_feedback failed");

        assert_eq!(response.message_id, message_id);
        assert_eq!(response.chunk_id, None);
        assert_eq!(response.sentiment, "positive");
    }

    #[tokio::test]
    async fn should_create_feedback_with_chunk_and_comment() {
        let store = Arc::new(temp_store().await);
        let message_id = sample_message_id(&store).await;
        let document = store
            .insert_document(NewDocument {
                source: DocumentSource::Manual,
                source_ref: "orari.md".into(),
                content: "Lo sportello apre alle 9:00".into(),
                metadata: None,
                embedding: vec![0.0; EMBEDDING_DIM],
            })
            .await
            .expect("insert_document failed");
        let adapter = KbStoreTrainingFeedbackAdapter::new(store);

        let response = adapter
            .create_feedback(NewTrainingFeedback {
                message_id,
                chunk_id: Some(document.id),
                answer_span: "alle 9:00".into(),
                sentiment: Sentiment::Negative,
                comment: Some("orario sbagliato".into()),
            })
            .await
            .expect("create_feedback failed");

        assert_eq!(response.chunk_id, Some(document.id));
        assert_eq!(response.sentiment, "negative");
        assert_eq!(response.comment.as_deref(), Some("orario sbagliato"));
    }

    #[tokio::test]
    async fn should_list_feedback_oldest_first() {
        let store = Arc::new(temp_store().await);
        let message_id = sample_message_id(&store).await;
        let adapter = KbStoreTrainingFeedbackAdapter::new(store);

        adapter
            .create_feedback(NewTrainingFeedback {
                message_id,
                chunk_id: None,
                answer_span: "prima porzione".into(),
                sentiment: Sentiment::Positive,
                comment: None,
            })
            .await
            .expect("create failed");
        adapter
            .create_feedback(NewTrainingFeedback {
                message_id,
                chunk_id: None,
                answer_span: "seconda porzione".into(),
                sentiment: Sentiment::Negative,
                comment: None,
            })
            .await
            .expect("create failed");

        let feedback = adapter
            .list_feedback(message_id)
            .await
            .expect("list_feedback failed");
        assert_eq!(feedback.len(), 2);
        assert_eq!(feedback[0].answer_span, "prima porzione");
        assert_eq!(feedback[1].answer_span, "seconda porzione");
    }
}
