use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub actor: String,
    pub created_at: DateTime<Utc>,
}

pub struct SessionStore {
    entries: Arc<DashMap<String, SessionRecord>>,
    ttl_secs: i64,
}

impl SessionStore {
    pub fn new(ttl_secs: i64) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            ttl_secs,
        }
    }

    pub fn insert(&self, actor: String) -> String {
        let token = generate_token();
        self.entries.insert(
            token.clone(),
            SessionRecord {
                actor,
                created_at: Utc::now(),
            },
        );
        token
    }

    pub fn get(&self, token: &str) -> Option<SessionRecord> {
        let entry = self.entries.get(token)?;

        let age_secs = (Utc::now() - entry.created_at).num_seconds();
        if age_secs > self.ttl_secs {
            drop(entry);
            self.entries.remove(token);
            return None;
        }

        Some(entry.clone())
    }

    pub fn remove(&self, token: &str) {
        self.entries.remove(token);
    }
}

fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| format!("{:02x}", rng.r#gen::<u8>()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_insert_and_retrieve_session() {
        let store = SessionStore::new(1800);
        let token = store.insert("operator".into());

        let record = store.get(&token).expect("session should exist");
        assert_eq!(record.actor, "operator");
    }

    #[test]
    fn should_return_none_for_missing_token() {
        let store = SessionStore::new(1800);
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn should_return_none_for_expired_session() {
        let store = SessionStore::new(0);
        let token = store.insert("operator".into());
        if let Some(mut record) = store.entries.get_mut(&token) {
            record.created_at = Utc::now() - chrono::Duration::seconds(1);
        }

        assert!(store.get(&token).is_none());
    }

    #[test]
    fn should_remove_session() {
        let store = SessionStore::new(1800);
        let token = store.insert("operator".into());

        store.remove(&token);

        assert!(store.get(&token).is_none());
    }

    #[test]
    fn should_generate_unique_tokens() {
        let store = SessionStore::new(1800);
        let token1 = store.insert("operator".into());
        let token2 = store.insert("operator".into());
        assert_ne!(token1, token2);
        assert_eq!(token1.len(), 64);
    }
}
