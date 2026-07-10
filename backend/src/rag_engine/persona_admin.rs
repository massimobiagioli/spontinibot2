use std::sync::Arc;

use async_trait::async_trait;
use kb_store::KbStore;

use crate::rag_engine::ports::{PersonaAdminPort, PersonaPort};
use crate::rag_engine::types::{AdminPersonaSnapshot, NewPersonaRequest, RagError};

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

impl From<kb_store::Persona> for AdminPersonaSnapshot {
    fn from(p: kb_store::Persona) -> Self {
        Self {
            id: p.id,
            name: p.name,
            version: p.version,
            system_prompt: p.system_prompt,
            tone: p.tone,
            fallback_message: p.fallback_message,
            is_active: p.is_active,
            created_at: p.created_at,
            created_by: p.created_by,
        }
    }
}

impl From<NewPersonaRequest> for kb_store::NewPersona {
    fn from(req: NewPersonaRequest) -> Self {
        Self {
            name: req.name,
            system_prompt: req.system_prompt,
            tone: req.tone,
            fallback_message: req.fallback_message,
            created_by: req.created_by,
        }
    }
}

#[async_trait]
impl PersonaAdminPort for PersonaAdminAdapter {
    async fn list_versions(&self, name: &str) -> Result<Vec<AdminPersonaSnapshot>, RagError> {
        self.store
            .get_persona_versions(name)
            .await
            .map(|v| v.into_iter().map(AdminPersonaSnapshot::from).collect())
            .map_err(|e| RagError::Persona(e.to_string()))
    }

    async fn insert_persona(
        &self,
        req: NewPersonaRequest,
    ) -> Result<AdminPersonaSnapshot, RagError> {
        let activate = req.activate;
        let new_persona: kb_store::NewPersona = req.into();
        let result = self
            .store
            .insert_persona(new_persona, activate)
            .await
            .map(AdminPersonaSnapshot::from)
            .map_err(|e| RagError::Persona(e.to_string()))?;

        if activate {
            self.persona_port.reload_persona().await?;
        }

        Ok(result)
    }

    async fn activate_persona(&self, id: i64) -> Result<(), RagError> {
        self.store.activate_persona(id).await.map_err(|e| {
            if matches!(e, kb_store::KbStoreError::NotFound(_)) {
                RagError::PersonaNotFound
            } else {
                RagError::Persona(e.to_string())
            }
        })?;

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

    fn sample_request(name: &str) -> NewPersonaRequest {
        NewPersonaRequest {
            name: name.into(),
            system_prompt: format!("Sei {name}."),
            tone: None,
            fallback_message: None,
            activate: false,
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
            .insert_persona(sample_request("gaspare"))
            .await
            .unwrap();
        admin
            .insert_persona(sample_request("gaspare"))
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

        let mut req1 = sample_request("gaspare");
        req1.activate = true;
        let p1 = admin.insert_persona(req1).await.unwrap();
        assert!(p1.is_active);

        let mut req2 = sample_request("gaspare");
        req2.activate = true;
        let p2 = admin.insert_persona(req2).await.unwrap();
        assert!(p2.is_active);

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

        let mut req1 = sample_request("gaspare");
        req1.activate = true;
        let p1 = admin.insert_persona(req1).await.unwrap();
        let p2 = admin
            .insert_persona(sample_request("gaspare"))
            .await
            .unwrap();

        assert!(p1.is_active);
        assert!(!p2.is_active);

        admin.activate_persona(p2.id).await.unwrap();

        let versions = admin.list_versions("gaspare").await.unwrap();
        assert!(versions[0].is_active);
        assert!(!versions[1].is_active);
    }

    #[tokio::test]
    async fn should_return_persona_not_found_when_activating_missing_id() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());
        let persona_port: Arc<dyn PersonaPort> = Arc::new(PersonaAdapter::new(store.clone()));
        let admin = PersonaAdminAdapter::new(store.clone(), persona_port);

        let result = admin.activate_persona(999).await;
        assert!(matches!(result, Err(RagError::PersonaNotFound)));

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_reload_persona_cache_on_insert_with_activate() {
        let path = temp_db_path();
        let store = Arc::new(KbStore::open(&path).await.unwrap());
        let persona_port: Arc<dyn PersonaPort> = Arc::new(PersonaAdapter::new(store.clone()));
        let admin = PersonaAdminAdapter::new(store.clone(), persona_port);

        admin
            .insert_persona(NewPersonaRequest {
                name: "gaspare".into(),
                system_prompt: "Version 1".into(),
                tone: None,
                fallback_message: None,
                activate: true,
                created_by: Some("admin".into()),
            })
            .await
            .unwrap();

        let snap: Option<PersonaSnapshot> = admin.persona_port.active_persona().await.unwrap();
        assert_eq!(snap.unwrap().system_prompt, "Version 1");

        admin
            .insert_persona(NewPersonaRequest {
                name: "gaspare".into(),
                system_prompt: "Version 2".into(),
                tone: None,
                fallback_message: None,
                activate: true,
                created_by: Some("admin".into()),
            })
            .await
            .unwrap();

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

        admin
            .insert_persona(NewPersonaRequest {
                name: "gaspare".into(),
                system_prompt: "Version 1".into(),
                tone: None,
                fallback_message: None,
                activate: true,
                created_by: Some("admin".into()),
            })
            .await
            .unwrap();

        let p2 = admin
            .insert_persona(NewPersonaRequest {
                name: "gaspare".into(),
                system_prompt: "Version 2".into(),
                tone: None,
                fallback_message: None,
                activate: false,
                created_by: Some("admin".into()),
            })
            .await
            .unwrap();

        let snap = admin.persona_port.active_persona().await.unwrap();
        assert_eq!(snap.unwrap().system_prompt, "Version 1");

        admin.activate_persona(p2.id).await.unwrap();

        let snap = admin.persona_port.active_persona().await.unwrap();
        assert_eq!(snap.unwrap().system_prompt, "Version 2");

        drop(store);
        let _ = std::fs::remove_file(&path);
    }
}
