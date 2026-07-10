use std::sync::Arc;

use async_trait::async_trait;

use kb_store::{KbStore, NewIngestSchedule, NewIngestSection, NewIngestSource};

use super::{
    IngestConfigAdminPort, IngestConfigError, IngestScheduleResponse, IngestSectionResponse,
    IngestSourceResponse,
};

pub struct KbStoreIngestConfigAdapter {
    store: Arc<KbStore>,
}

impl KbStoreIngestConfigAdapter {
    pub fn new(store: Arc<KbStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl IngestConfigAdminPort for KbStoreIngestConfigAdapter {
    async fn get_schedule(&self) -> Result<Option<IngestScheduleResponse>, IngestConfigError> {
        let schedule = self.store.get_schedule().await?;
        Ok(schedule.map(IngestScheduleResponse::from))
    }

    async fn upsert_schedule(
        &self,
        schedule: NewIngestSchedule,
    ) -> Result<IngestScheduleResponse, IngestConfigError> {
        let saved = self.store.upsert_schedule(schedule).await?;
        Ok(IngestScheduleResponse::from(saved))
    }

    async fn list_sections(&self) -> Result<Vec<IngestSectionResponse>, IngestConfigError> {
        let sections = self.store.list_sections().await?;
        Ok(sections
            .into_iter()
            .map(IngestSectionResponse::from)
            .collect())
    }

    async fn create_section(
        &self,
        section: NewIngestSection,
    ) -> Result<IngestSectionResponse, IngestConfigError> {
        let saved = self.store.upsert_section(section).await?;
        Ok(IngestSectionResponse::from(saved))
    }

    async fn delete_section(&self, id: i64) -> Result<bool, IngestConfigError> {
        let deleted = self.store.delete_section(id).await?;
        Ok(deleted)
    }

    async fn list_sources(
        &self,
        section_id: i64,
    ) -> Result<Vec<IngestSourceResponse>, IngestConfigError> {
        let sources = self.store.list_sources_by_section(section_id).await?;
        Ok(sources
            .into_iter()
            .map(IngestSourceResponse::from)
            .collect())
    }

    async fn create_source(
        &self,
        _section_id: i64,
        source: NewIngestSource,
    ) -> Result<IngestSourceResponse, IngestConfigError> {
        let saved = self.store.upsert_source(source).await?;
        Ok(IngestSourceResponse::from(saved))
    }

    async fn delete_source(&self, id: i64) -> Result<bool, IngestConfigError> {
        let deleted = self.store.delete_source(id).await?;
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kb_store::SourceType;
    use std::sync::atomic::{AtomicU32, Ordering};

    static DB_COUNTER: AtomicU32 = AtomicU32::new(1);

    async fn temp_store() -> KbStore {
        let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir();
        let path = dir.join(format!("ingest_config_adapter_test_{n}.db"));
        let _ = std::fs::remove_file(&path);
        KbStore::open(&path.to_string_lossy())
            .await
            .expect("failed to open temp db")
    }

    fn sample_schedule() -> NewIngestSchedule {
        NewIngestSchedule {
            cron_expr: "0 */4 * * *".into(),
            enabled: true,
        }
    }

    fn sample_section(name: &str, ordering: i32) -> NewIngestSection {
        NewIngestSection {
            name: name.into(),
            ordering,
        }
    }

    fn sample_scrape_source(section_id: i64) -> NewIngestSource {
        NewIngestSource {
            section_id,
            source_type: SourceType::Scrape,
            url: "https://example.com".into(),
            enabled: true,
        }
    }

    fn sample_api_source(section_id: i64) -> NewIngestSource {
        NewIngestSource {
            section_id,
            source_type: SourceType::Api,
            url: "https://api.example.com".into(),
            enabled: true,
        }
    }

    #[tokio::test]
    async fn should_return_none_when_no_schedule_exists() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreIngestConfigAdapter::new(store);
        let result = adapter.get_schedule().await.expect("get_schedule failed");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn should_upsert_and_return_schedule() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreIngestConfigAdapter::new(store);

        let response = adapter
            .upsert_schedule(sample_schedule())
            .await
            .expect("upsert_schedule failed");
        assert_eq!(response.cron_expr, "0 */4 * * *");
        assert!(response.enabled);

        let fetched = adapter.get_schedule().await.expect("get_schedule failed");
        assert!(fetched.is_some_and(|s| s.cron_expr == "0 */4 * * *"));
    }

    #[tokio::test]
    async fn should_create_section_and_return_it() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreIngestConfigAdapter::new(store);

        let response = adapter
            .create_section(sample_section("sport", 10))
            .await
            .expect("create_section failed");
        assert_eq!(response.name, "sport");
        assert_eq!(response.ordering, 10);
        assert!(response.id > 0);
    }

    #[tokio::test]
    async fn should_list_sections_in_order() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreIngestConfigAdapter::new(store);

        adapter
            .create_section(sample_section("news", 20))
            .await
            .unwrap();
        adapter
            .create_section(sample_section("sport", 10))
            .await
            .unwrap();

        let sections = adapter.list_sections().await.expect("list_sections failed");
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].name, "sport");
        assert_eq!(sections[1].name, "news");
    }

    #[tokio::test]
    async fn should_delete_section_and_return_true() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreIngestConfigAdapter::new(store);

        let section = adapter
            .create_section(sample_section("sport", 10))
            .await
            .unwrap();
        let deleted = adapter
            .delete_section(section.id)
            .await
            .expect("delete_section failed");
        assert!(deleted);

        let sections = adapter.list_sections().await.expect("list_sections failed");
        assert!(sections.is_empty());
    }

    #[tokio::test]
    async fn should_return_false_when_deleting_nonexistent_section() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreIngestConfigAdapter::new(store);

        let deleted = adapter
            .delete_section(999)
            .await
            .expect("delete_section failed");
        assert!(!deleted);
    }

    #[tokio::test]
    async fn should_create_source_and_return_it() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreIngestConfigAdapter::new(store);

        let section = adapter
            .create_section(sample_section("sport", 10))
            .await
            .unwrap();
        let response = adapter
            .create_source(section.id, sample_scrape_source(section.id))
            .await
            .expect("create_source failed");
        assert_eq!(response.section_id, section.id);
        assert_eq!(response.source_type, "scrape");
        assert!(response.enabled);
        assert!(!response.coming_soon);
    }

    #[tokio::test]
    async fn should_return_coming_soon_for_api_source() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreIngestConfigAdapter::new(store);

        let section = adapter
            .create_section(sample_section("news", 10))
            .await
            .unwrap();
        let response = adapter
            .create_source(section.id, sample_api_source(section.id))
            .await
            .expect("create_source failed");
        assert!(!response.enabled);
        assert!(response.coming_soon);
        assert_eq!(response.source_type, "api");
    }

    #[tokio::test]
    async fn should_list_sources_for_section() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreIngestConfigAdapter::new(store);

        let section = adapter
            .create_section(sample_section("sport", 10))
            .await
            .unwrap();
        adapter
            .create_source(section.id, sample_scrape_source(section.id))
            .await
            .unwrap();
        adapter
            .create_source(section.id, sample_api_source(section.id))
            .await
            .unwrap();

        let sources = adapter
            .list_sources(section.id)
            .await
            .expect("list_sources failed");
        assert_eq!(sources.len(), 2);
        assert!(sources.iter().any(|s| s.source_type == "scrape"));
        assert!(sources.iter().any(|s| s.source_type == "api"));
    }

    #[tokio::test]
    async fn should_delete_source_and_return_true() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreIngestConfigAdapter::new(store);

        let section = adapter
            .create_section(sample_section("sport", 10))
            .await
            .unwrap();
        let source = adapter
            .create_source(section.id, sample_scrape_source(section.id))
            .await
            .unwrap();
        let deleted = adapter
            .delete_source(source.id)
            .await
            .expect("delete_source failed");
        assert!(deleted);

        let sources = adapter
            .list_sources(section.id)
            .await
            .expect("list_sources failed");
        assert!(sources.is_empty());
    }

    #[tokio::test]
    async fn should_return_false_when_deleting_nonexistent_source() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreIngestConfigAdapter::new(store);

        let deleted = adapter
            .delete_source(999)
            .await
            .expect("delete_source failed");
        assert!(!deleted);
    }
}
