use std::sync::Arc;

use async_trait::async_trait;
use ingest_core::pipeline::IngestPipeline;

use super::UploadError;
use super::ports::UploadPort;
use super::preview_store::UploadMetadata;

pub struct IngestCoreUploadAdapter {
    pipeline: Arc<IngestPipeline>,
}

impl IngestCoreUploadAdapter {
    pub fn new(pipeline: Arc<IngestPipeline>) -> Self {
        Self { pipeline }
    }
}

#[async_trait]
impl UploadPort for IngestCoreUploadAdapter {
    async fn ingest_uploaded(
        &self,
        text: &str,
        section: &str,
        filename: &str,
        metadata: &UploadMetadata,
    ) -> Result<Vec<i64>, UploadError> {
        let metadata_json = serde_json::json!({
            "category": metadata.category,
            "tags": metadata.tags,
            "trust_score": metadata.trust_score,
        });
        let metadata_str = Some(metadata_json.to_string());

        self.pipeline
            .process_manual_upload(text, section, filename, metadata_str)
            .await
            .map_err(|e| UploadError::IngestFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingest_core::pipeline::IngestPipeline;
    use kb_store::KbStore;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn should_delegate_to_pipeline_and_return_ids() {
        let embed_server = MockServer::start().await;

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

        let dir = std::env::temp_dir();
        let path = dir
            .join(format!(
                "upload_adapter_test_{}.db",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .to_string_lossy()
            .into_owned();

        let kb = KbStore::open(&path).await.expect("failed to open db");
        let pipeline = Arc::new(
            IngestPipeline::new("test-agent".into(), embed_server.uri(), 512, 64, kb)
                .expect("failed to create pipeline"),
        );

        let adapter = IngestCoreUploadAdapter::new(pipeline);
        let metadata = UploadMetadata {
            category: Some("test".into()),
            tags: Some(vec!["tag1".into()]),
            trust_score: Some(0.9),
        };

        let ids = adapter
            .ingest_uploaded(
                "Test content for upload adapter.",
                "news",
                "test.txt",
                &metadata,
            )
            .await
            .expect("ingest failed");

        assert!(!ids.is_empty(), "expected at least one document ID");

        let _ = std::fs::remove_file(&path);
    }
}
