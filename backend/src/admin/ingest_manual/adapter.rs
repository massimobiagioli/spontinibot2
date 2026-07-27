use std::sync::Arc;

use async_trait::async_trait;

use ingest_core::error::IngestError;
use ingest_core::pipeline::{IngestPipeline, Pipeline};

use super::{IngestManualAdminPort, IngestManualError, IngestManualResponse, RecencyWindow};

impl From<IngestError> for IngestManualError {
    fn from(e: IngestError) -> Self {
        match e {
            IngestError::RobotsTxt(msg) => IngestManualError::RobotsTxt(msg),
            other => IngestManualError::Ingest(other.to_string()),
        }
    }
}

pub struct PipelineIngestManualAdapter {
    pipeline: Arc<IngestPipeline>,
}

impl PipelineIngestManualAdapter {
    pub fn new(pipeline: Arc<IngestPipeline>) -> Self {
        Self { pipeline }
    }
}

#[async_trait]
impl IngestManualAdminPort for PipelineIngestManualAdapter {
    async fn ingest(
        &self,
        section: &str,
        src: &str,
        window: RecencyWindow,
    ) -> Result<IngestManualResponse, IngestManualError> {
        self.pipeline.run(src, section).await?;
        Ok(IngestManualResponse {
            section: section.to_string(),
            src: src.to_string(),
            window: window.to_string(),
            status: "ingested".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kb_store::KbStore;
    use std::sync::atomic::{AtomicU32, Ordering};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    static DB_COUNTER: AtomicU32 = AtomicU32::new(1);

    fn temp_db_path() -> String {
        let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir()
            .join(format!("ingest_manual_adapter_test_{n}.db"))
            .to_string_lossy()
            .into_owned()
    }

    async fn new_pipeline(embed_server_uri: &str, kb: KbStore) -> Arc<IngestPipeline> {
        Arc::new(
            IngestPipeline::new(
                "backend-manual-ingest-test".into(),
                embed_server_uri.into(),
                512,
                64,
                kb,
            )
            .expect("failed to create pipeline"),
        )
    }

    #[tokio::test]
    async fn should_ingest_successfully_via_shared_pipeline() {
        let mock_server = MockServer::start().await;
        let embed_server = MockServer::start().await;

        let html =
            "<html><body><h1>Title</h1><p>Some real page content for the test.</p></body></html>";
        Mock::given(method("GET"))
            .and(path("/page"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(html.as_bytes().to_vec(), "text/html"),
            )
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let embedding_768: Vec<f32> = (0..768).map(|i| i as f32 * 0.001).collect();
        let nested = serde_json::json!([{ "index": 0, "embedding": [embedding_768] }]);
        Mock::given(method("POST"))
            .and(path("/embedding"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&nested))
            .mount(&embed_server)
            .await;

        let db_path = temp_db_path();
        let kb = KbStore::open(&db_path).await.expect("failed to open db");
        let pipeline = new_pipeline(&embed_server.uri(), kb).await;
        let adapter = PipelineIngestManualAdapter::new(pipeline);

        let url = format!("{}/page", mock_server.uri());
        let result = adapter
            .ingest("storia", &url, RecencyWindow::Days(30))
            .await
            .expect("ingest failed");

        assert_eq!(result.section, "storia");
        assert_eq!(result.src, url);
        assert_eq!(result.window, "30d");
        assert_eq!(result.status, "ingested");

        let _ = std::fs::remove_file(&db_path);
    }

    #[tokio::test]
    async fn should_return_robots_txt_error_not_a_panic_when_disallowed() {
        let mock_server = MockServer::start().await;
        let embed_server = MockServer::start().await;

        let robots = "User-agent: *\nDisallow: /\n";
        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(robots)
                    .insert_header("content-type", "text/plain"),
            )
            .mount(&mock_server)
            .await;

        let db_path = temp_db_path();
        let kb = KbStore::open(&db_path).await.expect("failed to open db");
        let pipeline = new_pipeline(&embed_server.uri(), kb).await;
        let adapter = PipelineIngestManualAdapter::new(pipeline);

        let url = format!("{}/atti-amministrativi/delibere", mock_server.uri());
        let result = adapter
            .ingest("delibere", &url, RecencyWindow::Days(30))
            .await;

        assert!(
            matches!(result, Err(IngestManualError::RobotsTxt(_))),
            "expected RobotsTxt error, got {result:?}"
        );

        let _ = std::fs::remove_file(&db_path);
    }
}
