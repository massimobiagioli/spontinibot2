use std::sync::Arc;

use async_trait::async_trait;

use kb_store::{KbStore, NewTrainingSession};

use super::{TrainingSessionAdminPort, TrainingSessionError, TrainingSessionResponse};

pub struct KbStoreTrainingSessionAdapter {
    store: Arc<KbStore>,
}

impl KbStoreTrainingSessionAdapter {
    pub fn new(store: Arc<KbStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl TrainingSessionAdminPort for KbStoreTrainingSessionAdapter {
    async fn create_session(
        &self,
        req: NewTrainingSession,
    ) -> Result<TrainingSessionResponse, TrainingSessionError> {
        let session = self.store.create_training_session(req).await?;
        Ok(TrainingSessionResponse::from(session))
    }

    async fn list_sessions(&self) -> Result<Vec<TrainingSessionResponse>, TrainingSessionError> {
        let sessions = self.store.list_training_sessions().await?;
        Ok(sessions
            .into_iter()
            .map(TrainingSessionResponse::from)
            .collect())
    }

    async fn get_session(
        &self,
        id: i64,
    ) -> Result<Option<TrainingSessionResponse>, TrainingSessionError> {
        let session = self.store.get_training_session(id).await?;
        Ok(session.map(TrainingSessionResponse::from))
    }

    async fn close_session(
        &self,
        id: i64,
        notes: Option<String>,
    ) -> Result<bool, TrainingSessionError> {
        let closed = self.store.close_training_session(id, notes).await?;
        Ok(closed)
    }

    async fn delete_session(&self, id: i64) -> Result<bool, TrainingSessionError> {
        let deleted = self.store.delete_training_session(id).await?;
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static DB_COUNTER: AtomicU32 = AtomicU32::new(1);

    async fn temp_store() -> KbStore {
        let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir();
        let path = dir.join(format!("training_session_adapter_test_{n}.db"));
        let _ = std::fs::remove_file(&path);
        KbStore::open(&path.to_string_lossy())
            .await
            .expect("failed to open temp db")
    }

    fn sample_session(title: &str) -> NewTrainingSession {
        NewTrainingSession {
            title: title.into(),
            created_by: None,
        }
    }

    #[tokio::test]
    async fn should_create_session_and_return_it_open() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreTrainingSessionAdapter::new(store);

        let response = adapter
            .create_session(sample_session("Sessione 1"))
            .await
            .expect("create_session failed");
        assert_eq!(response.title, "Sessione 1");
        assert!(response.closed_at.is_none());
    }

    #[tokio::test]
    async fn should_list_sessions_newest_first() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreTrainingSessionAdapter::new(store);

        adapter
            .create_session(sample_session("First"))
            .await
            .unwrap();
        let second = adapter
            .create_session(sample_session("Second"))
            .await
            .unwrap();

        let sessions = adapter.list_sessions().await.expect("list_sessions failed");
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, second.id);
    }

    #[tokio::test]
    async fn should_return_none_for_unknown_session_id() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreTrainingSessionAdapter::new(store);

        let result = adapter.get_session(999).await.expect("get_session failed");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn should_close_open_session_and_return_true() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreTrainingSessionAdapter::new(store);

        let created = adapter
            .create_session(sample_session("Sessione"))
            .await
            .unwrap();

        let closed = adapter
            .close_session(created.id, None)
            .await
            .expect("close_session failed");
        assert!(closed);

        let fetched = adapter
            .get_session(created.id)
            .await
            .expect("get_session failed")
            .expect("should find the session");
        assert!(fetched.closed_at.is_some());
    }

    #[tokio::test]
    async fn should_return_false_when_closing_already_closed_session() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreTrainingSessionAdapter::new(store);

        let created = adapter
            .create_session(sample_session("Sessione"))
            .await
            .unwrap();
        adapter.close_session(created.id, None).await.unwrap();

        let second_close = adapter
            .close_session(created.id, None)
            .await
            .expect("close_session failed");
        assert!(!second_close);
    }

    #[tokio::test]
    async fn should_delete_session_and_return_true() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreTrainingSessionAdapter::new(store);

        let created = adapter
            .create_session(sample_session("Sessione"))
            .await
            .unwrap();

        let deleted = adapter
            .delete_session(created.id)
            .await
            .expect("delete_session failed");
        assert!(deleted);

        let fetched = adapter
            .get_session(created.id)
            .await
            .expect("get_session failed");
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn should_return_false_when_deleting_unknown_session() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreTrainingSessionAdapter::new(store);

        let deleted = adapter
            .delete_session(999)
            .await
            .expect("delete_session failed");
        assert!(!deleted);
    }
}
