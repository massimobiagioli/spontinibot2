use ingest_core::pipeline::Pipeline;
use kb_store::KbStore;

use crate::config::IngestSource;
use crate::error::IngestError;

pub struct PipelineRunner {
    pipeline: Box<dyn Pipeline>,
}

impl PipelineRunner {
    pub fn new(pipeline: Box<dyn Pipeline>) -> Self {
        Self { pipeline }
    }

    pub async fn run_all(&self, sources: &[IngestSource]) -> Result<(), IngestError> {
        for src in sources {
            tracing::info!(
                "running pipeline for source: section={}, url={}",
                src.section,
                src.url
            );

            if let Err(e) = self.pipeline.run(&src.url, &src.section).await {
                tracing::error!(
                    "pipeline failed for section={}, url={}: {e}",
                    src.section,
                    src.url
                );
            }
        }
        Ok(())
    }
}

pub fn create_pipeline(
    user_agent: String,
    embedder_base_url: String,
    chunk_size: usize,
    chunk_overlap: usize,
    kb: KbStore,
) -> Result<Box<dyn Pipeline>, IngestError> {
    ingest_core::pipeline::IngestPipeline::new(
        user_agent,
        embedder_base_url,
        chunk_size,
        chunk_overlap,
        kb,
    )
    .map(|p| Box::new(p) as Box<dyn Pipeline>)
    .map_err(|e| IngestError::Pipeline(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    static DB_COUNTER: AtomicU32 = AtomicU32::new(1);

    fn temp_db_path() -> String {
        let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir();
        dir.join(format!("ingest_runner_integration_{n}.db"))
            .to_string_lossy()
            .into_owned()
    }

    #[tokio::test]
    async fn should_scrape_chunk_embed_and_store_via_runner() {
        let mock_server = MockServer::start().await;
        let embed_server = MockServer::start().await;

        let html = "<html><body><h1>Runner Test</h1><p>Integration test content.</p></body></html>";
        Mock::given(method("GET"))
            .and(path("/page"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(html.to_string().into_bytes(), "text/html"),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let embedding_768: Vec<f32> = (0..768).map(|i| i as f32 * 0.001).collect();
        let nested = serde_json::json!([{
            "index": 0,
            "embedding": [embedding_768]
        }]);
        Mock::given(method("POST"))
            .and(path("/embedding"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&nested))
            .mount(&embed_server)
            .await;

        let path = temp_db_path();
        {
            let kb = KbStore::open(&path).await.expect("failed to open db");
            let pipeline = create_pipeline("test-agent".into(), embed_server.uri(), 512, 64, kb)
                .expect("failed to create pipeline");

            let runner = PipelineRunner::new(pipeline);

            let sources = vec![crate::config::IngestSource {
                section: "test-section".into(),
                url: format!("{}/page", mock_server.uri()),
            }];

            runner
                .run_all(&sources)
                .await
                .expect("runner run_all failed");
        }

        let kb = KbStore::open(&path).await.expect("failed to re-open db");
        let docs = kb
            .get_documents_by_source(kb_store::DocumentSource::Scrape, 10, 0)
            .await
            .expect("get docs failed");
        assert!(!docs.is_empty(), "expected at least one document");
        assert!(
            docs.iter().any(|d| d.content.contains("Runner Test")),
            "expected document content to contain 'Runner Test'"
        );

        let _ = std::fs::remove_file(&path);
    }
}
