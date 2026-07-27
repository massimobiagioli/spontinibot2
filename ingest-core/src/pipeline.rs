use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::chunking::Chunker;
use crate::embed::EmbeddingClient;
use crate::error::IngestError;
use crate::scraper::ScraperAdapter;
use kb_store::{DocumentSource, KbStore, NewDocument};

pub fn version() -> &'static str {
    "pipeline module 0.1.0"
}

#[async_trait]
pub trait Pipeline: Send + Sync {
    async fn run(&self, url: &str, section: &str) -> Result<(), IngestError>;
}

pub struct IngestPipeline {
    scraper: Mutex<ScraperAdapter>,
    chunker: Chunker,
    embedder: EmbeddingClient,
    kb: KbStore,
}

impl IngestPipeline {
    pub fn new(
        user_agent: String,
        embedder_base_url: String,
        chunk_size: usize,
        chunk_overlap: usize,
        kb: KbStore,
    ) -> Result<Self, IngestError> {
        Ok(Self {
            scraper: Mutex::new(ScraperAdapter::new(user_agent)),
            chunker: Chunker::new(chunk_size, chunk_overlap)?,
            embedder: EmbeddingClient::new(embedder_base_url),
            kb,
        })
    }
}

#[async_trait]
impl Pipeline for IngestPipeline {
    async fn run(&self, url: &str, section: &str) -> Result<(), IngestError> {
        let text = self.scraper.lock().await.fetch_text(url).await?;
        let chunks = self.chunker.chunk(&text, section, url)?;

        for chunk in &chunks {
            let embedding = self.embedder.embed_chunk(&chunk.content).await?;
            let doc = NewDocument {
                source: DocumentSource::Scrape,
                source_ref: url.to_string(),
                content: chunk.content.clone(),
                metadata: chunk.metadata.clone(),
                embedding,
                section: Some(section.to_string()),
            };
            self.kb.insert_document(doc).await?;
        }

        Ok(())
    }
}

impl IngestPipeline {
    /// Process a manually uploaded document: chunk the text, embed each chunk,
    /// and insert into the knowledge base with `DocumentSource::Manual`.
    ///
    /// Returns the IDs of the inserted documents.
    pub async fn process_manual_upload(
        &self,
        text: &str,
        section: &str,
        filename: &str,
        metadata: Option<String>,
    ) -> Result<Vec<i64>, IngestError> {
        let chunks = self.chunker.chunk(text, section, filename)?;
        let mut document_ids = Vec::new();

        for chunk in &chunks {
            let embedding = self.embedder.embed_chunk(&chunk.content).await?;
            let doc = NewDocument {
                source: DocumentSource::Manual,
                source_ref: filename.to_string(),
                content: chunk.content.clone(),
                metadata: metadata.clone().or_else(|| chunk.metadata.clone()),
                embedding,
                section: Some(section.to_string()),
            };
            let inserted = self.kb.insert_document(doc).await?;
            document_ids.push(inserted.id);
        }

        Ok(document_ids)
    }
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
        dir.join(format!("ingest_pipeline_test_{n}.db"))
            .to_string_lossy()
            .into_owned()
    }

    #[tokio::test]
    async fn should_scrape_chunk_embed_and_store_documents() {
        let mock_server = MockServer::start().await;
        let embed_server = MockServer::start().await;

        let html = "<html><body><h1>Title</h1><p>Content of the page.</p></body></html>";
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
            let pipeline =
                IngestPipeline::new("test-agent".into(), embed_server.uri(), 512, 64, kb)
                    .expect("failed to create pipeline");

            let url = format!("{}/page", mock_server.uri());
            pipeline
                .run(&url, "test-section")
                .await
                .expect("pipeline run failed");
        }

        let kb = KbStore::open(&path).await.expect("failed to re-open db");
        let docs = kb
            .get_documents_by_source(DocumentSource::Scrape, 10, 0)
            .await
            .expect("get docs failed");
        assert!(!docs.is_empty(), "expected at least one document");
        assert!(
            docs.iter().any(|d| d.content.contains("Title")),
            "expected document content to contain Title"
        );
        assert!(
            docs.iter()
                .all(|d| d.section.as_deref() == Some("test-section")),
            "expected every scraped document to carry its section name"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn should_return_version_when_called() {
        assert_eq!(version(), "pipeline module 0.1.0");
    }

    #[test]
    fn pipeline_is_object_safe() {
        // Compile-time assertion: this compiles only if Pipeline is object-safe
        let _p: Option<Box<dyn Pipeline>> = None;
    }

    #[tokio::test]
    async fn should_return_robots_txt_error_when_url_disallowed_by_robots() {
        let mock_server = MockServer::start().await;
        let embed_server = MockServer::start().await;

        let robots = "User-agent: *\nDisallow: /private/\n";
        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(robots)
                    .insert_header("content-type", "text/plain"),
            )
            .mount(&mock_server)
            .await;

        let path = temp_db_path();
        let kb = KbStore::open(&path).await.expect("failed to open db");
        let pipeline = IngestPipeline::new("test-agent".into(), embed_server.uri(), 512, 64, kb)
            .expect("failed to create pipeline");

        let url = format!("{}/private/data", mock_server.uri());
        let result = pipeline.run(&url, "test-section").await;

        assert!(
            matches!(result, Err(IngestError::RobotsTxt(_))),
            "expected RobotsTxt error, got {result:?}"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_chunk_embed_and_store_manual_upload() {
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

        let path = temp_db_path();
        {
            let kb = KbStore::open(&path).await.expect("failed to open db");
            let pipeline =
                IngestPipeline::new("test-agent".into(), embed_server.uri(), 512, 64, kb)
                    .expect("failed to create pipeline");

            let text = "This is a manual upload test document with enough content to fill a chunk properly.";
            let ids = pipeline
                .process_manual_upload(text, "news", "test.pdf", None)
                .await
                .expect("manual upload failed");

            assert!(!ids.is_empty(), "expected at least one document ID");
        }

        let kb = KbStore::open(&path).await.expect("failed to re-open db");
        let docs = kb
            .get_documents_by_source(DocumentSource::Manual, 10, 0)
            .await
            .expect("get docs failed");
        assert!(!docs.is_empty(), "expected at least one manual document");
        assert_eq!(docs[0].source_ref, "test.pdf");
        assert_eq!(docs[0].section.as_deref(), Some("news"));
        assert!(
            docs[0].content.contains("manual upload"),
            "expected document content to contain upload text"
        );

        let _ = std::fs::remove_file(&path);
    }
}
