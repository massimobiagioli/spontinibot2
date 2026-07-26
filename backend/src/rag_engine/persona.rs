use std::sync::Arc;

use async_trait::async_trait;
use kb_store::KbStore;
use tokio::sync::RwLock;

use crate::rag_engine::ports::PersonaPort;
use crate::rag_engine::types::{PersonaSnapshot, RagError};

pub struct PersonaAdapter {
    store: Arc<KbStore>,
    cache: RwLock<Option<PersonaSnapshot>>,
}

impl PersonaAdapter {
    pub fn new(store: Arc<KbStore>) -> Self {
        Self {
            store,
            cache: RwLock::new(None),
        }
    }
}

#[async_trait]
impl PersonaPort for PersonaAdapter {
    async fn active_persona(&self) -> Result<Option<PersonaSnapshot>, RagError> {
        {
            let cache = self.cache.read().await;
            if let Some(snapshot) = &*cache {
                return Ok(Some(snapshot.clone()));
            }
        }

        let persona = self
            .store
            .get_active_persona()
            .await
            .map_err(|e| RagError::Persona(e.to_string()))?;

        let snapshot = persona.map(|p| PersonaSnapshot {
            name: p.name,
            system_prompt: p.system_prompt,
            fallback_message: p.fallback_message,
        });

        *self.cache.write().await = snapshot.clone();
        Ok(snapshot)
    }

    async fn reload_persona(&self) -> Result<(), RagError> {
        *self.cache.write().await = None;
        Ok(())
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
        assert_eq!(s.name, "gaspare");
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

    #[tokio::test]
    async fn should_reload_cache_when_reload_called() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());

        store
            .insert_persona(
                NewPersona {
                    name: "gaspare".into(),
                    system_prompt: "Versione 1".into(),
                    tone: None,
                    fallback_message: None,
                    created_by: Some("admin".into()),
                },
                true,
            )
            .await
            .unwrap();

        let adapter = PersonaAdapter::new(store.clone());

        // First call — cache miss, queries DB
        let s1 = adapter.active_persona().await.unwrap();
        assert_eq!(s1.unwrap().system_prompt, "Versione 1");

        // Insert a new active persona via store (bypasses cache)
        store
            .insert_persona(
                NewPersona {
                    name: "gaspare".into(),
                    system_prompt: "Versione 2".into(),
                    tone: None,
                    fallback_message: None,
                    created_by: Some("admin".into()),
                },
                true,
            )
            .await
            .unwrap();

        // Second call — cache HIT, still returns old version
        let s2 = adapter.active_persona().await.unwrap();
        assert_eq!(s2.unwrap().system_prompt, "Versione 1");

        // Reload — clear cache
        adapter.reload_persona().await.unwrap();

        // Third call — cache miss, re-queries DB, returns new version
        let s3 = adapter.active_persona().await.unwrap();
        assert_eq!(s3.unwrap().system_prompt, "Versione 2");

        drop(store);
        let _ = std::fs::remove_file(&path);
    }
}
