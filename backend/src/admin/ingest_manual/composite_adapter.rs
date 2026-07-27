//! Dispatches a manual-ingest request to either the existing scrape pipeline
//! (feature 0029, robots.txt-honoring, unmodified) or the explicit,
//! config-driven Halley curation path (Plan 0030), based on `src`'s host.
//! Both sides implement the same `IngestManualAdminPort` — this is Open/
//! Closed extension, not a change to either existing adapter.

use std::sync::Arc;

use async_trait::async_trait;
use url::Url;

use super::{IngestManualAdminPort, IngestManualError, IngestManualResponse, RecencyWindow};

pub struct CuratingIngestManualAdapter {
    scrape: Arc<dyn IngestManualAdminPort>,
    curation: Arc<dyn IngestManualAdminPort>,
    curation_allowed_hosts: Vec<String>,
}

impl CuratingIngestManualAdapter {
    pub fn new(
        scrape: Arc<dyn IngestManualAdminPort>,
        curation: Arc<dyn IngestManualAdminPort>,
        curation_allowed_hosts: Vec<String>,
    ) -> Self {
        Self {
            scrape,
            curation,
            curation_allowed_hosts,
        }
    }

    fn is_curation_host(&self, src: &str) -> bool {
        let Some(host) = Url::parse(src)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
        else {
            return false;
        };
        self.curation_allowed_hosts
            .iter()
            .any(|allowed| &host == allowed || host.ends_with(&format!(".{allowed}")))
    }
}

#[async_trait]
impl IngestManualAdminPort for CuratingIngestManualAdapter {
    async fn ingest(
        &self,
        section: &str,
        src: &str,
        window: RecencyWindow,
    ) -> Result<IngestManualResponse, IngestManualError> {
        if self.is_curation_host(src) {
            self.curation.ingest(section, src, window).await
        } else {
            self.scrape.ingest(section, src, window).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct SpyPort {
        calls: AtomicUsize,
        response_status: &'static str,
    }

    impl SpyPort {
        fn new(response_status: &'static str) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                response_status,
            }
        }
    }

    #[async_trait]
    impl IngestManualAdminPort for SpyPort {
        async fn ingest(
            &self,
            section: &str,
            src: &str,
            window: RecencyWindow,
        ) -> Result<IngestManualResponse, IngestManualError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(IngestManualResponse {
                section: section.to_string(),
                src: src.to_string(),
                window: window.to_string(),
                status: self.response_status.to_string(),
            })
        }
    }

    #[tokio::test]
    async fn should_dispatch_allowlisted_host_to_curation() {
        let scrape = Arc::new(SpyPort::new("scrape"));
        let curation = Arc::new(SpyPort::new("curation"));
        let adapter = CuratingIngestManualAdapter::new(
            scrape.clone(),
            curation.clone(),
            vec!["halleyweb.com".to_string()],
        );

        let response = adapter
            .ingest(
                "delibere",
                "https://www.halleyweb.com/c042023/zf/index.php/atti-amministrativi/delibere",
                RecencyWindow::Days(30),
            )
            .await
            .expect("ingest failed");

        assert_eq!(response.status, "curation");
        assert_eq!(curation.calls.load(Ordering::SeqCst), 1);
        assert_eq!(scrape.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn should_dispatch_other_hosts_to_scrape() {
        let scrape = Arc::new(SpyPort::new("scrape"));
        let curation = Arc::new(SpyPort::new("curation"));
        let adapter = CuratingIngestManualAdapter::new(
            scrape.clone(),
            curation.clone(),
            vec!["halleyweb.com".to_string()],
        );

        let response = adapter
            .ingest(
                "storia",
                "https://it.wikipedia.org/wiki/Maiolati_Spontini",
                RecencyWindow::Days(30),
            )
            .await
            .expect("ingest failed");

        assert_eq!(response.status, "scrape");
        assert_eq!(scrape.calls.load(Ordering::SeqCst), 1);
        assert_eq!(curation.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn should_dispatch_to_scrape_when_url_is_unparseable() {
        let scrape = Arc::new(SpyPort::new("scrape"));
        let curation = Arc::new(SpyPort::new("curation"));
        let adapter = CuratingIngestManualAdapter::new(
            scrape.clone(),
            curation.clone(),
            vec!["halleyweb.com".to_string()],
        );

        let _ = adapter
            .ingest("storia", "not a url", RecencyWindow::Days(30))
            .await;

        assert_eq!(scrape.calls.load(Ordering::SeqCst), 1);
        assert_eq!(curation.calls.load(Ordering::SeqCst), 0);
    }
}
