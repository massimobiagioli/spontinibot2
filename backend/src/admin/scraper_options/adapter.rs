use std::sync::Arc;

use async_trait::async_trait;

use kb_store::KbStore;

use super::{RobotsBypassHostResponse, ScraperOptionsAdminPort, ScraperOptionsError};

pub struct KbStoreScraperOptionsAdapter {
    store: Arc<KbStore>,
}

impl KbStoreScraperOptionsAdapter {
    pub fn new(store: Arc<KbStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl ScraperOptionsAdminPort for KbStoreScraperOptionsAdapter {
    async fn list_robots_bypass_hosts(
        &self,
    ) -> Result<Vec<RobotsBypassHostResponse>, ScraperOptionsError> {
        let hosts = self.store.list_robots_bypass_hosts().await?;
        Ok(hosts
            .into_iter()
            .map(RobotsBypassHostResponse::from)
            .collect())
    }

    async fn replace_robots_bypass_hosts(
        &self,
        hosts: Vec<String>,
    ) -> Result<Vec<RobotsBypassHostResponse>, ScraperOptionsError> {
        let saved = self.store.replace_robots_bypass_hosts(hosts).await?;
        Ok(saved
            .into_iter()
            .map(RobotsBypassHostResponse::from)
            .collect())
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
        let path = dir.join(format!("scraper_options_adapter_test_{n}.db"));
        let _ = std::fs::remove_file(&path);
        KbStore::open(&path.to_string_lossy())
            .await
            .expect("failed to open temp db")
    }

    #[tokio::test]
    async fn should_list_hosts_prepopulated_by_migration() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreScraperOptionsAdapter::new(store);

        let hosts = adapter
            .list_robots_bypass_hosts()
            .await
            .expect("list failed");
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].host, "www.comune.maiolatispontini.an.it");
    }

    #[tokio::test]
    async fn should_replace_hosts_wholesale_and_return_the_new_set() {
        let store = Arc::new(temp_store().await);
        let adapter = KbStoreScraperOptionsAdapter::new(store);

        let saved = adapter
            .replace_robots_bypass_hosts(vec!["a.example.com".into(), "b.example.com".into()])
            .await
            .expect("replace failed");
        assert_eq!(saved.len(), 2);
        assert_eq!(saved[0].host, "a.example.com");
        assert_eq!(saved[1].host, "b.example.com");

        let listed = adapter
            .list_robots_bypass_hosts()
            .await
            .expect("list failed");
        assert_eq!(listed.len(), 2);
    }
}
