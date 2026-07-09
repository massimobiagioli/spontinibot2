use serde::Deserialize;

use crate::error::IngestError;

pub fn version() -> &'static str {
    "embed module 0.1.0"
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<Vec<f32>>,
}

pub struct EmbeddingClient {
    client: reqwest::Client,
    base_url: String,
}

impl EmbeddingClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }

    pub async fn embed_chunk(&self, text: &str) -> Result<Vec<f32>, IngestError> {
        let url = format!("{}/embedding", self.base_url);
        let body = serde_json::json!({ "content": text });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| IngestError::Embedding(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(IngestError::Embedding(format!(
                "HTTP {status}: {body_text}"
            )));
        }

        let mut items: Vec<EmbeddingResponse> = resp
            .json()
            .await
            .map_err(|e| IngestError::Embedding(e.to_string()))?;

        if items.is_empty() {
            return Err(IngestError::Embedding("empty embedding response".into()));
        }

        let flat = items
            .pop()
            .unwrap()
            .embedding
            .into_iter()
            .next()
            .ok_or_else(|| IngestError::Embedding("missing inner embedding vector".into()))?;

        if flat.len() != kb_store::EMBEDDING_DIM {
            return Err(IngestError::DimensionMismatch {
                expected: kb_store::EMBEDDING_DIM,
                actual: flat.len(),
            });
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
    async fn should_return_embedding_vector_when_response_valid() {
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

        let client = EmbeddingClient::new(mock_server.uri());
        let result = client.embed_chunk("test text").await.unwrap();
        assert_eq!(result.len(), 768);
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

        let client = EmbeddingClient::new(mock_server.uri());
        let err = client.embed_chunk("test").await.unwrap_err();
        assert!(
            matches!(
                err,
                IngestError::DimensionMismatch {
                    expected: 768,
                    actual: 512
                }
            ),
            "expected DimensionMismatch, got {err:?}"
        );
    }

    #[tokio::test]
    async fn should_return_error_on_http_500() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embedding"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
            .mount(&mock_server)
            .await;

        let client = EmbeddingClient::new(mock_server.uri());
        let err = client.embed_chunk("test").await.unwrap_err();
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

        let client = EmbeddingClient::new(mock_server.uri());
        let err = client.embed_chunk("test").await.unwrap_err();
        assert!(
            err.to_string().contains("empty"),
            "expected empty error, got {err:?}"
        );
    }

    #[tokio::test]
    async fn should_return_error_on_malformed_json() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embedding"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json at all"))
            .mount(&mock_server)
            .await;

        let client = EmbeddingClient::new(mock_server.uri());
        let err = client.embed_chunk("test").await.unwrap_err();
        assert!(
            matches!(err, IngestError::Embedding(_)),
            "expected Embedding error, got {err:?}"
        );
    }
}
