use std::sync::Arc;

use async_trait::async_trait;
use kb_store::KbStore;

use crate::rag_engine::ports::PersonaPort;
use crate::rag_engine::types::{PersonaSnapshot, RagError};

pub struct PersonaAdapter {
    store: Arc<KbStore>,
}

impl PersonaAdapter {
    pub fn new(store: Arc<KbStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl PersonaPort for PersonaAdapter {
    async fn active_persona(&self) -> Result<Option<PersonaSnapshot>, RagError> {
        let persona = self
            .store
            .get_active_persona()
            .await
            .map_err(|e| RagError::Persona(e.to_string()))?;

        Ok(persona.map(|p| PersonaSnapshot {
            system_prompt: p.system_prompt,
            fallback_message: p.fallback_message,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kb_store::NewPersona;
    use std::sync::atomic::{AtomicU32, Ordering};

    static DB_COUNTER: AtomicU32 = AtomicU32::new(100);

    fn temp_db_path() -> String {
        let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir()
            .join(format!("persona_adapter_test_{n}.db"))
            .to_string_lossy()
            .into_owned()
    }

    fn sample_persona(name: &str) -> NewPersona {
        NewPersona {
            name: name.into(),
            system_prompt: "Sei Gaspare Spontini.".into(),
            tone: Some("caldo".into()),
            fallback_message: Some("Non ho trovato l'informazione.".into()),
            created_by: Some("admin".into()),
        }
    }

    #[tokio::test]
    async fn should_return_snapshot_when_active_persona_exists() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());
        store
            .insert_persona(sample_persona("gaspare"), true)
            .await
            .unwrap();

        let adapter = PersonaAdapter::new(store.clone());
        let snapshot = adapter.active_persona().await.unwrap();

        let s = snapshot.expect("should have active persona");
        assert_eq!(s.system_prompt, "Sei Gaspare Spontini.");
        assert_eq!(
            s.fallback_message.as_deref(),
            Some("Non ho trovato l'informazione.")
        );

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_none_when_no_active_persona() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());
        store
            .insert_persona(sample_persona("gaspare"), false)
            .await
            .unwrap();

        let adapter = PersonaAdapter::new(store.clone());
        let snapshot = adapter.active_persona().await.unwrap();

        assert!(snapshot.is_none());

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_return_none_when_persona_table_empty() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());

        let adapter = PersonaAdapter::new(store.clone());
        let snapshot = adapter.active_persona().await.unwrap();

        assert!(snapshot.is_none());

        drop(store);
        let _ = std::fs::remove_file(&path);
    }
}
