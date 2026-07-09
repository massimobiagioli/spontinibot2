use std::sync::Arc;

use kb_store::{IngestSchedule, IngestSection, KbStore, SourceType};
use tokio::sync::watch;

use crate::error::IngestError;

#[derive(Clone, Debug, PartialEq)]
pub struct IngestSource {
    pub section: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IngestConfig {
    pub cron_expression: String,
    pub schedule_enabled: bool,
    pub sections: Vec<String>,
    pub sources: Vec<IngestSource>,
}

pub struct ConfigLoader {
    kb_path: String,
}

impl ConfigLoader {
    pub fn new(kb_path: String) -> Self {
        Self { kb_path }
    }

    pub async fn load(&self) -> Result<IngestConfig, IngestError> {
        let kb = KbStore::open(&self.kb_path)
            .await
            .map_err(|e| IngestError::KbStore(e.to_string()))?;

        let schedule: Option<IngestSchedule> = kb
            .get_schedule()
            .await
            .map_err(|e| IngestError::KbStore(e.to_string()))?;

        let schedule_enabled = schedule.as_ref().map(|s| s.enabled).unwrap_or(false);
        let cron_expression = schedule
            .as_ref()
            .map(|s| s.cron_expr.clone())
            .unwrap_or_default();

        if schedule_enabled && cron_expression.is_empty() {
            return Err(IngestError::Config(
                "schedule is enabled but cron expression is empty".into(),
            ));
        }

        let sections: Vec<IngestSection> = kb
            .list_sections()
            .await
            .map_err(|e| IngestError::KbStore(e.to_string()))?;

        let section_names: Vec<String> = sections.iter().map(|s| s.name.clone()).collect();

        let mut sources = Vec::new();
        for section in &sections {
            let section_sources = kb
                .list_sources_by_section(section.id)
                .await
                .map_err(|e| IngestError::KbStore(e.to_string()))?;

            for src in section_sources {
                if src.enabled && src.source_type == SourceType::Scrape {
                    sources.push(IngestSource {
                        section: section.name.clone(),
                        url: src.url.clone(),
                    });
                }
            }
        }

        Ok(IngestConfig {
            cron_expression,
            schedule_enabled,
            sections: section_names,
            sources,
        })
    }
}

pub struct ConfigWatcher {
    loader: Arc<ConfigLoader>,
    poll_secs: u64,
}

impl ConfigWatcher {
    pub fn new(loader: ConfigLoader, poll_secs: u64) -> Self {
        Self {
            loader: Arc::new(loader),
            poll_secs,
        }
    }

    pub async fn run(
        self,
    ) -> (
        watch::Receiver<Option<IngestConfig>>,
        tokio::task::JoinHandle<()>,
    ) {
        let (tx, rx) = watch::channel(None);

        let loader = self.loader;
        let poll_secs = self.poll_secs;

        let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            let mut last_config: Option<IngestConfig> = None;

            loop {
                match loader.load().await {
                    Ok(config) => {
                        if Some(&config) != last_config.as_ref() {
                            last_config = Some(config.clone());
                            let _ = tx.send(Some(config));
                        }
                    }
                    Err(e) => {
                        tracing::warn!("config poll failed: {e}");
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(poll_secs)).await;
            }
        });

        (rx, handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kb_store::{NewIngestSchedule, NewIngestSection, NewIngestSource};
    use std::sync::atomic::{AtomicU32, Ordering};

    static DB_COUNTER: AtomicU32 = AtomicU32::new(1);

    fn temp_db_path() -> String {
        let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir();
        dir.join(format!("ingest_config_test_{n}.db"))
            .to_string_lossy()
            .into_owned()
    }

    #[tokio::test]
    async fn should_return_empty_config_when_no_schedule() {
        let path = temp_db_path();
        let _kb = KbStore::open(&path).await.expect("failed to open db");

        let loader = ConfigLoader::new(path.clone());
        let config = loader.load().await.expect("load failed");
        assert_eq!(config.cron_expression, "");
        assert!(!config.schedule_enabled);
        assert!(config.sections.is_empty());
        assert!(config.sources.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_load_schedule_and_sections_and_sources() {
        let path = temp_db_path();
        let kb = KbStore::open(&path).await.expect("failed to open db");

        kb.upsert_schedule(NewIngestSchedule {
            cron_expr: "0 0 */4 * * * *".into(),
            enabled: true,
        })
        .await
        .expect("upsert schedule");

        let sec = kb
            .upsert_section(NewIngestSection {
                name: "news".into(),
                ordering: 0,
            })
            .await
            .expect("upsert section");

        kb.upsert_source(NewIngestSource {
            section_id: sec.id,
            source_type: SourceType::Scrape,
            url: "https://example.com/news".into(),
            enabled: true,
        })
        .await
        .expect("upsert source");

        drop(kb);

        let loader = ConfigLoader::new(path.clone());
        let config = loader.load().await.expect("load failed");
        assert_eq!(config.cron_expression, "0 0 */4 * * * *");
        assert!(config.schedule_enabled);
        assert_eq!(config.sections, vec!["news"]);
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].section, "news");
        assert_eq!(config.sources[0].url, "https://example.com/news");

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_filter_to_only_enabled_scrape_sources() {
        let path = temp_db_path();
        let kb = KbStore::open(&path).await.expect("failed to open db");

        kb.upsert_schedule(NewIngestSchedule {
            cron_expr: "0 0 0 * * * *".into(),
            enabled: true,
        })
        .await
        .expect("upsert schedule");

        let sec = kb
            .upsert_section(NewIngestSection {
                name: "sport".into(),
                ordering: 0,
            })
            .await
            .expect("upsert section");

        kb.upsert_source(NewIngestSource {
            section_id: sec.id,
            source_type: SourceType::Scrape,
            url: "https://example.com/sport-enabled".into(),
            enabled: true,
        })
        .await
        .expect("upsert source");

        kb.upsert_source(NewIngestSource {
            section_id: sec.id,
            source_type: SourceType::Scrape,
            url: "https://example.com/sport-disabled".into(),
            enabled: false,
        })
        .await
        .expect("upsert source");

        kb.upsert_source(NewIngestSource {
            section_id: sec.id,
            source_type: SourceType::Api,
            url: "https://api.example.com".into(),
            enabled: true,
        })
        .await
        .expect("upsert source");

        drop(kb);

        let loader = ConfigLoader::new(path.clone());
        let config = loader.load().await.expect("load failed");
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].url, "https://example.com/sport-enabled");

        let _ = std::fs::remove_file(&path);
    }
}
