use std::sync::Arc;

use async_trait::async_trait;

use kb_store::KbStore;

use super::{IngestRunAdminPort, IngestRunError, IngestRunResponse};

pub struct KbStoreIngestRunAdapter {
    store: Arc<KbStore>,
}

impl KbStoreIngestRunAdapter {
    pub fn new(store: Arc<KbStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl IngestRunAdminPort for KbStoreIngestRunAdapter {
    async fn trigger_run(&self) -> Result<IngestRunResponse, IngestRunError> {
        let request = self.store.request_run().await?;
        Ok(IngestRunResponse::from(request))
    }

    async fn get_run(&self, id: i64) -> Result<Option<IngestRunResponse>, IngestRunError> {
        let request = self.store.get_run_request(id).await?;
        Ok(request.map(IngestRunResponse::from))
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
        let path = dir.join(format!("ingest_run_adapter_test_{n}.db"));
        let _ = std::fs::remove_file(&path);
        KbStore::open(&path.to_string_lossy())
            .await
            .expect("failed to open temp db")
    }

    #[tokio::test]
    async fn should_trigger_run_and_return_pending() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreIngestRunAdapter::new(store);

        let response = adapter.trigger_run().await.expect("trigger_run failed");
        assert!(response.id > 0);
        assert_eq!(response.status, "pending");
    }

    #[tokio::test]
    async fn should_get_run_by_id_after_trigger() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreIngestRunAdapter::new(store);

        let triggered = adapter.trigger_run().await.expect("trigger_run failed");
        let fetched = adapter
            .get_run(triggered.id)
            .await
            .expect("get_run failed")
            .expect("should find the run");

        assert_eq!(fetched.id, triggered.id);
        assert_eq!(fetched.status, "pending");
    }

    #[tokio::test]
    async fn should_return_none_for_unknown_run_id() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreIngestRunAdapter::new(store);

        let result = adapter.get_run(999).await.expect("get_run failed");
        assert!(result.is_none());
    }
}
