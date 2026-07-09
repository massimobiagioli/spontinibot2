use async_trait::async_trait;
use serde::Deserialize;

use crate::rag_engine::ports::EmbeddingPort;
use crate::rag_engine::types::RagError;

/// `llama.cpp` server `/embedding` endpoint contract (verified 2026-07-09):
///
/// Request: `POST /embedding` with `{"content": "<text>"}`
///
/// Response:
/// ```json
/// [{"index": 0, "embedding": [[0.01, -0.02, ...]]}]
/// ```
///
/// The `embedding` field is a nested array: `[[], []]` — outer list
/// contains one element per input, inner list is the 768-dim float vector.
pub struct EmbeddingAdapter {
    client: reqwest::Client,
    base_url: String,
}

impl EmbeddingAdapter {
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<Vec<f32>>,
}

#[async_trait]
impl EmbeddingPort for EmbeddingAdapter {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, RagError> {
        let url = format!("{}/embedding", self.base_url);
        let body = serde_json::json!({ "content": text });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| RagError::Embedding(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(RagError::Embedding(format!("HTTP {status}: {body_text}")));
        }

        let mut items: Vec<EmbeddingResponse> = resp
            .json()
            .await
            .map_err(|e| RagError::Embedding(e.to_string()))?;

        if items.is_empty() {
            return Err(RagError::Embedding("empty embedding response".into()));
        }

        let flat = items
            .pop()
            .unwrap()
            .embedding
            .into_iter()
            .next()
            .ok_or_else(|| RagError::Embedding("missing inner embedding vector".into()))?;

        if flat.len() != kb_store::EMBEDDING_DIM {
            return Err(RagError::Embedding(format!(
                "expected {} dimensions, got {}",
                kb_store::EMBEDDING_DIM,
                flat.len()
            )));
        }

        Ok(flat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn should_extract_flat_vector_from_nested_response() {
        let mock_server = MockServer::start().await;

        let embedding_768: Vec<f32> = (0..768).map(|i| i as f32 * 0.001).collect();
        let nested = serde_json::json!([{
            "index": 0,
            "embedding": [embedding_768]
        }]);

        Mock::given(method("POST"))
            .and(path("/embedding"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&nested))
            .expect(1)
            .mount(&mock_server)
            .await;

        let adapter = EmbeddingAdapter::new(mock_server.uri());
        let result = adapter.embed("test text").await.unwrap();

        assert_eq!(result.len(), 768);
        assert!((result[0] - 0.0).abs() < f32::EPSILON);
        assert!((result[1] - 0.001).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn should_reject_wrong_dimension() {
        let mock_server = MockServer::start().await;

        let bad_embedding: Vec<f32> = vec![0.1; 512];
        let nested = serde_json::json!([{
            "index": 0,
            "embedding": [bad_embedding]
        }]);

        Mock::given(method("POST"))
            .and(path("/embedding"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&nested))
            .mount(&mock_server)
            .await;

        let adapter = EmbeddingAdapter::new(mock_server.uri());
        let err = adapter.embed("test").await.unwrap_err();

        assert!(matches!(err, RagError::Embedding(_)));
        assert!(err.to_string().contains("512"));
    }

    #[tokio::test]
    async fn should_return_error_on_server_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embedding"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
            .mount(&mock_server)
            .await;

        let adapter = EmbeddingAdapter::new(mock_server.uri());
        let err = adapter.embed("test").await.unwrap_err();

        assert!(matches!(err, RagError::Embedding(_)));
        assert!(err.to_string().contains("500"));
    }

    #[tokio::test]
    async fn should_return_error_on_empty_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embedding"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&mock_server)
            .await;

        let adapter = EmbeddingAdapter::new(mock_server.uri());
        let err = adapter.embed("test").await.unwrap_err();

        assert!(matches!(err, RagError::Embedding(_)));
        assert!(err.to_string().contains("empty"));
    }
}
