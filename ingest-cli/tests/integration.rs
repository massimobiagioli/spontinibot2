use std::sync::atomic::{AtomicU32, Ordering};

use ingest_core::pipeline::{IngestPipeline, Pipeline};
use kb_store::{DocumentSource, KbStore, NewIngestSection, NewIngestSource, SourceType};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

static DB_COUNTER: AtomicU32 = AtomicU32::new(1);

fn temp_db_path() -> String {
    let n = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir();
    dir.join(format!("ingest_cli_int_test_{n}.db"))
        .to_string_lossy()
        .into_owned()
}

fn embedding_response() -> serde_json::Value {
    let embedding_768: Vec<f32> = (0..768).map(|i| i as f32 * 0.001).collect();
    serde_json::json!([{
        "index": 0,
        "embedding": [embedding_768]
    }])
}

async fn setup_mock_source(server: &MockServer, page_path: &str) {
    let html = "<html><body><h1>Title</h1><p>Content of the page.</p></body></html>";
    Mock::given(method("GET"))
        .and(path(page_path))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(html.to_string().into_bytes(), "text/html"),
        )
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(404))
        .mount(server)
        .await;
}

async fn setup_embedder(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/embedding"))
        .respond_with(ResponseTemplate::new(200).set_body_json(embedding_response()))
        .mount(server)
        .await;
}

#[tokio::test]
async fn should_ingest_single_url_via_run_url() {
    let source_server = MockServer::start().await;
    let embed_server = MockServer::start().await;

    setup_mock_source(&source_server, "/page").await;
    setup_embedder(&embed_server).await;

    let path = temp_db_path();
    let url = format!("{}/page", source_server.uri());

    {
        let kb = KbStore::open(&path).await.expect("failed to open db");
        let pipeline = IngestPipeline::new("test-agent".into(), embed_server.uri(), 512, 64, kb)
            .expect("failed to create pipeline");

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
        "expected document content to contain 'Title'"
    );

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn should_ingest_only_enabled_sources_for_section() {
    let source_server = MockServer::start().await;
    let embed_server = MockServer::start().await;

    setup_mock_source(&source_server, "/page").await;
    setup_embedder(&embed_server).await;

    let path = temp_db_path();

    let kb = KbStore::open(&path).await.expect("failed to open db");
    let section = kb
        .upsert_section(NewIngestSection {
            name: "test-section".into(),
            ordering: 0,
        })
        .await
        .expect("failed to create section");

    let enabled_url = format!("{}/page", source_server.uri());
    kb.upsert_source(NewIngestSource {
        section_id: section.id,
        source_type: SourceType::Scrape,
        url: enabled_url,
        enabled: true,
    })
    .await
    .expect("failed to create enabled source");

    kb.upsert_source(NewIngestSource {
        section_id: section.id,
        source_type: SourceType::Scrape,
        url: "http://localhost:9999/disabled".into(),
        enabled: false,
    })
    .await
    .expect("failed to create disabled source");

    let sections = kb.list_sections().await.expect("list sections failed");
    let sec = sections.iter().find(|s| s.name == "test-section").unwrap();
    let sources = kb
        .list_sources_by_section(sec.id)
        .await
        .expect("list sources failed");
    let enabled: Vec<_> = sources
        .iter()
        .filter(|s| s.enabled && s.source_type == SourceType::Scrape)
        .collect();

    assert_eq!(enabled.len(), 1, "expected exactly 1 enabled source");

    let pipeline = IngestPipeline::new("test-agent".into(), embed_server.uri(), 512, 64, kb)
        .expect("failed to create pipeline");

    for source in &enabled {
        pipeline
            .run(&source.url, "test-section")
            .await
            .expect("pipeline run failed");
    }

    drop(pipeline);

    let kb3 = KbStore::open(&path).await.expect("failed to re-open db");
    let docs = kb3
        .get_documents_by_source(DocumentSource::Scrape, 10, 0)
        .await
        .expect("get docs failed");
    assert!(!docs.is_empty(), "expected at least one document");
    assert!(
        docs.iter().any(|d| d.content.contains("Title")),
        "expected document content to contain 'Title'"
    );

    let _ = std::fs::remove_file(&path);
}
