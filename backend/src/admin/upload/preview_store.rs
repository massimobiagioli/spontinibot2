use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use rand::Rng;

use super::{ExtractedText, UploadError};

#[derive(Debug, Clone)]
pub struct UploadMetadata {
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub trust_score: Option<f32>,
    pub summary: Option<String>,
    /// The publicly reachable page this document was sourced from (e.g. the
    /// Halley "atti-amministrativi" detail page for a delibera). `None` for
    /// manual uploads with no known public URL — the citation then falls
    /// back to the document title instead of a link.
    pub source_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PreviewEntry {
    pub extracted_text: ExtractedText,
    pub section: String,
    pub metadata: UploadMetadata,
    pub filename: String,
    pub created_at: DateTime<Utc>,
}

pub struct PreviewStore {
    entries: Arc<DashMap<String, PreviewEntry>>,
    ttl_minutes: i64,
}

impl PreviewStore {
    pub fn new(ttl_minutes: i64) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            ttl_minutes,
        }
    }

    pub fn insert(&self, entry: PreviewEntry) -> String {
        let token = generate_token();
        self.entries.insert(token.clone(), entry);
        token
    }

    pub fn get(&self, token: &str) -> Result<PreviewEntry, UploadError> {
        let entry = self
            .entries
            .get(token)
            .ok_or(UploadError::PreviewNotFound)?;

        let now = Utc::now();
        let age_minutes = (now - entry.created_at).num_minutes();
        if age_minutes > self.ttl_minutes {
            drop(entry);
            self.entries.remove(token);
            return Err(UploadError::PreviewNotFound);
        }

        Ok(entry.clone())
    }

    pub fn remove(&self, token: &str) -> Result<PreviewEntry, UploadError> {
        let (_, entry) = self
            .entries
            .remove(token)
            .ok_or(UploadError::PreviewNotFound)?;
        Ok(entry)
    }

    pub fn evict_expired(&self) {
        let now = Utc::now();
        let ttl = self.ttl_minutes;
        self.entries
            .retain(|_, entry| (now - entry.created_at).num_minutes() <= ttl);
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
    use crate::admin::upload::DocumentFormat;

    fn sample_entry() -> PreviewEntry {
        PreviewEntry {
            extracted_text: ExtractedText {
                content: "test content".into(),
                format: DocumentFormat::PlainText,
                byte_size: 12,
            },
            section: "news".into(),
            metadata: UploadMetadata {
                category: None,
                tags: None,
                trust_score: None,
                summary: None,
                source_url: None,
            },
            filename: "test.txt".into(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn should_insert_and_retrieve_entry() {
        let store = PreviewStore::new(15);
        let entry = sample_entry();
        let token = store.insert(entry.clone());

        let retrieved = store.get(&token).unwrap();
        assert_eq!(retrieved.extracted_text.content, "test content");
        assert_eq!(retrieved.section, "news");
    }

    #[test]
    fn should_return_error_for_missing_token() {
        let store = PreviewStore::new(15);
        let result = store.get("nonexistent");
        assert!(matches!(result, Err(UploadError::PreviewNotFound)));
    }

    #[test]
    fn should_remove_entry() {
        let store = PreviewStore::new(15);
        let token = store.insert(sample_entry());

        let removed = store.remove(&token).unwrap();
        assert_eq!(removed.extracted_text.content, "test content");

        let result = store.get(&token);
        assert!(matches!(result, Err(UploadError::PreviewNotFound)));
    }

    #[test]
    fn should_return_error_for_expired_entry() {
        let store = PreviewStore::new(0);
        let mut entry = sample_entry();
        entry.created_at = Utc::now() - chrono::Duration::minutes(1);
        let token = store.insert(entry);

        let result = store.get(&token);
        assert!(matches!(result, Err(UploadError::PreviewNotFound)));
    }

    #[test]
    fn should_evict_expired_entries() {
        let store = PreviewStore::new(0);
        let mut entry = sample_entry();
        entry.created_at = Utc::now() - chrono::Duration::minutes(1);
        let token = store.insert(entry);

        store.evict_expired();

        let result = store.get(&token);
        assert!(matches!(result, Err(UploadError::PreviewNotFound)));
    }

    #[test]
    fn should_generate_unique_tokens() {
        let store = PreviewStore::new(15);
        let token1 = store.insert(sample_entry());
        let token2 = store.insert(sample_entry());
        assert_ne!(token1, token2);
        assert_eq!(token1.len(), 64);
    }
}
