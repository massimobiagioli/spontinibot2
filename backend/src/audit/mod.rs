pub mod adapter;

use std::fmt;

use async_trait::async_trait;

#[derive(Debug)]
pub enum AuditError {
    DbError(String),
}

impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditError::DbError(msg) => write!(f, "database error: {msg}"),
        }
    }
}

impl std::error::Error for AuditError {}

impl From<kb_store::KbStoreError> for AuditError {
    fn from(e: kb_store::KbStoreError) -> Self {
        AuditError::DbError(e.to_string())
    }
}

#[async_trait]
pub trait AuditLogPort: Send + Sync {
    async fn record(
        &self,
        actor: &str,
        action: &str,
        target: &str,
        payload: &serde_json::Value,
    ) -> Result<(), AuditError>;
}

/// Records an audit entry best-effort: the operator's write already
/// succeeded, so a failure to record it is logged, not surfaced as a
/// request failure (the audit trail is not transactional with the write).
pub async fn record_best_effort(
    audit: &dyn AuditLogPort,
    actor: &str,
    action: &str,
    target: &str,
    payload: &serde_json::Value,
) {
    if let Err(e) = audit.record(actor, action, target, payload).await {
        tracing::error!("failed to record audit entry for action {action} on {target}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct RecordingAudit {
        calls: Mutex<Vec<(String, String, String)>>,
    }

    #[async_trait]
    impl AuditLogPort for RecordingAudit {
        async fn record(
            &self,
            actor: &str,
            action: &str,
            target: &str,
            _payload: &serde_json::Value,
        ) -> Result<(), AuditError> {
            self.calls.lock().unwrap().push((
                actor.to_string(),
                action.to_string(),
                target.to_string(),
            ));
            Ok(())
        }
    }

    struct FailingAudit;

    #[async_trait]
    impl AuditLogPort for FailingAudit {
        async fn record(
            &self,
            _actor: &str,
            _action: &str,
            _target: &str,
            _payload: &serde_json::Value,
        ) -> Result<(), AuditError> {
            Err(AuditError::DbError("connection refused".into()))
        }
    }

    #[tokio::test]
    async fn should_call_through_on_success() {
        let audit = RecordingAudit {
            calls: Mutex::new(Vec::new()),
        };

        record_best_effort(
            &audit,
            "operator",
            "create_persona",
            "persona:1",
            &serde_json::json!({}),
        )
        .await;

        let calls = audit.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "operator");
        assert_eq!(calls[0].1, "create_persona");
        assert_eq!(calls[0].2, "persona:1");
    }

    #[tokio::test]
    async fn should_not_panic_when_audit_write_fails() {
        let audit = FailingAudit;

        record_best_effort(
            &audit,
            "operator",
            "create_persona",
            "persona:1",
            &serde_json::json!({}),
        )
        .await;
    }
}
