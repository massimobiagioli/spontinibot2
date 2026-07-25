use std::sync::Arc;

use async_trait::async_trait;

use kb_store::{KbStore, NewAuditLogEntry};

use super::{AuditError, AuditLogPort};

pub struct KbStoreAuditLogAdapter {
    store: Arc<KbStore>,
}

impl KbStoreAuditLogAdapter {
    pub fn new(store: Arc<KbStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl AuditLogPort for KbStoreAuditLogAdapter {
    async fn record(
        &self,
        actor: &str,
        action: &str,
        target: &str,
        payload: &serde_json::Value,
    ) -> Result<(), AuditError> {
        self.store
            .insert_audit_entry(NewAuditLogEntry {
                actor: actor.to_string(),
                action: action.to_string(),
                target: target.to_string(),
                payload: payload.to_string(),
            })
            .await?;
        Ok(())
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
        let path = dir.join(format!("audit_adapter_test_{n}.db"));
        let _ = std::fs::remove_file(&path);
        KbStore::open(&path.to_string_lossy())
            .await
            .expect("failed to open temp db")
    }

    #[tokio::test]
    async fn should_record_entry_retrievable_via_kb_store() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreAuditLogAdapter::new(store.clone());

        adapter
            .record(
                "operator",
                "create_persona",
                "persona:1",
                &serde_json::json!({"name": "gaspare"}),
            )
            .await
            .expect("record failed");

        let entries = store
            .list_audit_entries()
            .await
            .expect("list_audit_entries failed");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].actor, "operator");
        assert_eq!(entries[0].action, "create_persona");
        assert_eq!(entries[0].target, "persona:1");
        assert_eq!(entries[0].payload, r#"{"name":"gaspare"}"#);
    }
}
