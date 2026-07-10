use std::sync::Arc;

use async_trait::async_trait;
use kb_store::KbStore;

use crate::rag_engine::ports::{PersonaAdminPort, PersonaPort};
use crate::rag_engine::types::RagError;

pub struct PersonaAdminAdapter {
    store: Arc<KbStore>,
    persona_port: Arc<dyn PersonaPort>,
}

impl PersonaAdminAdapter {
    pub fn new(store: Arc<KbStore>, persona_port: Arc<dyn PersonaPort>) -> Self {
        Self {
            store,
            persona_port,
        }
    }
}

#[async_trait]
impl PersonaAdminPort for PersonaAdminAdapter {
    async fn list_versions(&self, name: &str) -> Result<Vec<kb_store::Persona>, RagError> {
        self.store
            .get_persona_versions(name)
            .await
            .map_err(|e| RagError::Persona(e.to_string()))
    }

    async fn insert_persona(
        &self,
        persona: kb_store::NewPersona,
        activate: bool,
    ) -> Result<kb_store::Persona, RagError> {
        let result = self
            .store
            .insert_persona(persona, activate)
            .await
            .map_err(|e| RagError::Persona(e.to_string()))?;

        if activate {
            self.persona_port.reload_persona().await?;
        }

        Ok(result)
    }

    async fn activate_persona(&self, id: i64) -> Result<(), RagError> {
        self.store
            .activate_persona(id)
            .await
            .map_err(|e| RagError::Persona(e.to_string()))?;

        self.persona_port.reload_persona().await?;
        Ok(())
    }

    async fn reload_persona(&self) -> Result<(), RagError> {
        self.persona_port.reload_persona().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag_engine::persona::PersonaAdapter;
    use crate::rag_engine::types::PersonaSnapshot;
    use kb_store::NewPersona;
    use std::sync::atomic::{AtomicU32, Ordering};

    static DB_COUNTER: AtomicU32 = AtomicU32::new(200);

    fn temp_db_path() -> String {
        let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("persona_admin_test_{n}.db"))
            .to_string_lossy()
            .into_owned();
        let _ = std::fs::remove_file(&path);
        path
    }

    fn sample_new(name: &str) -> NewPersona {
        NewPersona {
            name: name.into(),
            system_prompt: format!("Sei {name}."),
            tone: None,
            fallback_message: None,
            created_by: Some("admin".into()),
        }
    }

    #[tokio::test]
    async fn should_list_versions() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());
        let persona_port: Arc<dyn PersonaPort> = Arc::new(PersonaAdapter::new(store.clone()));
        let admin = PersonaAdminAdapter::new(store.clone(), persona_port);

        admin
            .insert_persona(sample_new("gaspare"), false)
            .await
            .unwrap();
        admin
            .insert_persona(sample_new("gaspare"), false)
            .await
            .unwrap();

        let versions = admin.list_versions("gaspare").await.unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version, 2);
        assert_eq!(versions[1].version, 1);

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_insert_and_activate() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());
        let persona_port: Arc<dyn PersonaPort> = Arc::new(PersonaAdapter::new(store.clone()));
        let admin = PersonaAdminAdapter::new(store.clone(), persona_port);

        let p1 = admin
            .insert_persona(sample_new("gaspare"), true)
            .await
            .unwrap();
        assert!(p1.is_active);

        let p2 = admin
            .insert_persona(sample_new("gaspare"), true)
            .await
            .unwrap();
        assert!(p2.is_active);

        // versions[0] is version 2 (newest, active), versions[1] is version 1 (oldest, now inactive)
        let versions = admin.list_versions("gaspare").await.unwrap();
        assert!(versions[0].is_active);
        assert!(!versions[1].is_active);

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_activate_specific_version() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());
        let persona_port: Arc<dyn PersonaPort> = Arc::new(PersonaAdapter::new(store.clone()));
        let admin = PersonaAdminAdapter::new(store.clone(), persona_port);

        let p1 = admin
            .insert_persona(sample_new("gaspare"), true)
            .await
            .unwrap();
        let p2 = admin
            .insert_persona(sample_new("gaspare"), false)
            .await
            .unwrap();

        // p1 is active, p2 is not
        assert!(p1.is_active);
        assert!(!p2.is_active);

        // Activate p2
        admin.activate_persona(p2.id).await.unwrap();

        // versions[0] is version 2 (newest, now active), versions[1] is version 1 (oldest, now inactive)
        let versions = admin.list_versions("gaspare").await.unwrap();
        assert!(versions[0].is_active);
        assert!(!versions[1].is_active);
    }

    #[tokio::test]
    async fn should_reload_persona_cache_on_insert_with_activate() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());
        let persona_port: Arc<dyn PersonaPort> = Arc::new(PersonaAdapter::new(store.clone()));
        let admin = PersonaAdminAdapter::new(store.clone(), persona_port);

        admin
            .insert_persona(
                NewPersona {
                    name: "gaspare".into(),
                    system_prompt: "Version 1".into(),
                    tone: None,
                    fallback_message: None,
                    created_by: Some("admin".into()),
                },
                true,
            )
            .await
            .unwrap();

        // Active persona returns version 1 (cached)
        let snap: Option<PersonaSnapshot> = admin.persona_port.active_persona().await.unwrap();
        assert_eq!(snap.unwrap().system_prompt, "Version 1");

        // Insert new version with activate — should reload cache
        admin
            .insert_persona(
                NewPersona {
                    name: "gaspare".into(),
                    system_prompt: "Version 2".into(),
                    tone: None,
                    fallback_message: None,
                    created_by: Some("admin".into()),
                },
                true,
            )
            .await
            .unwrap();

        // Cache should be reloaded — returns version 2
        let snap: Option<PersonaSnapshot> = admin.persona_port.active_persona().await.unwrap();
        assert_eq!(snap.unwrap().system_prompt, "Version 2");

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_reload_persona_cache_on_activate() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());
        let persona_port: Arc<dyn PersonaPort> = Arc::new(PersonaAdapter::new(store.clone()));
        let admin = PersonaAdminAdapter::new(store.clone(), persona_port);

        let _p1 = admin
            .insert_persona(
                NewPersona {
                    name: "gaspare".into(),
                    system_prompt: "Version 1".into(),
                    tone: None,
                    fallback_message: None,
                    created_by: Some("admin".into()),
                },
                true,
            )
            .await
            .unwrap();

        let p2 = admin
            .insert_persona(
                NewPersona {
                    name: "gaspare".into(),
                    system_prompt: "Version 2".into(),
                    tone: None,
                    fallback_message: None,
                    created_by: Some("admin".into()),
                },
                false,
            )
            .await
            .unwrap();

        // Active is still version 1 (cached from insert)
        let snap = admin.persona_port.active_persona().await.unwrap();
        assert_eq!(snap.unwrap().system_prompt, "Version 1");

        // Activate version 2 — should reload cache
        admin.activate_persona(p2.id).await.unwrap();

        let snap = admin.persona_port.active_persona().await.unwrap();
        assert_eq!(snap.unwrap().system_prompt, "Version 2");

        drop(store);
        let _ = std::fs::remove_file(&path);
    }
}
