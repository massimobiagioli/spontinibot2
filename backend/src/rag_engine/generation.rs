use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::rag_engine::ports::GenerationPort;
use crate::rag_engine::types::{PromptParts, RagError};

/// `llama.cpp` server `/v1/chat/completions` endpoint contract (verified 2026-07-09):
///
/// Request:
/// ```json
/// {
///   "model": "qwen2.5-3b-instruct",
///   "messages": [
///     {"role": "system", "content": "<persona + citation instruction>"},
///     {"role": "user", "content": "<context>\\n<question>"}
///   ],
///   "stream": false,
///   "temperature": 0.3,
///   "max_tokens": 512
/// }
/// ```
///
/// Response (OpenAI-compatible):
/// ```json
/// {
///   "choices": [{"message": {"role": "assistant", "content": "<answer>"}}]
/// }
/// ```
pub struct GenerationAdapter {
    client: reqwest::Client,
    base_url: String,
}

const CITATION_INSTRUCTION: &str = "\
Rispondi UNICAMENTE usando il contesto fornito. \
Cita il documento di origine indicandone il titolo. \
Se il contesto non contiene la risposta, \
di' che non hai trovato l'informazione nei documenti comunali.";

impl GenerationAdapter {
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }

    fn assemble_messages(prompt: PromptParts) -> Vec<ChatMessage> {
        let system_content = format!("{}\n\n{}", prompt.system, CITATION_INSTRUCTION);

        let user_content = format!(
            "<context>\n{}\n</context>\n\n<question>\n{}\n</question>",
            prompt.context, prompt.user
        );

        vec![
            ChatMessage {
                role: "system".into(),
                content: system_content,
            },
            ChatMessage {
                role: "user".into(),
                content: user_content,
            },
        ]
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize)]
struct CompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Deserialize)]
struct CompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize)]
struct MessageContent {
    content: String,
}

#[async_trait]
impl GenerationPort for GenerationAdapter {
    async fn generate(&self, prompt: PromptParts) -> Result<String, RagError> {
        let messages = Self::assemble_messages(prompt);
        let body = CompletionRequest {
            model: "qwen2.5-3b-instruct".into(),
            messages,
            stream: false,
            temperature: 0.3,
            max_tokens: 512,
        };

        let url = format!("{}/v1/chat/completions", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| RagError::Generation(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(RagError::Generation(format!("HTTP {status}: {body_text}")));
        }

        let completion: CompletionResponse = resp
            .json()
            .await
            .map_err(|e| RagError::Generation(e.to_string()))?;

        completion
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| RagError::Generation("empty choices".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn should_extract_content_from_openai_compatible_response() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "choices": [{
                "message": {"role": "assistant", "content": "Lo sportello e' aperto 9-12:30."},
                "finish_reason": "stop",
                "index": 0
            }]
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let adapter = GenerationAdapter::new(mock_server.uri());
        let prompt = PromptParts {
            system: "Sei Gaspare Spontini.".into(),
            context: "[Fonte: orari.md]\nOrari: 9-12:30".into(),
            user: "A che ore apre l'anagrafe?".into(),
        };

        let answer = adapter.generate(prompt).await.unwrap();
        assert_eq!(answer, "Lo sportello e' aperto 9-12:30.");
    }

    #[tokio::test]
    async fn should_return_error_on_http_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
            .mount(&mock_server)
            .await;

        let adapter = GenerationAdapter::new(mock_server.uri());
        let prompt = PromptParts {
            system: "test".into(),
            context: "test".into(),
            user: "test".into(),
        };

        let err = adapter.generate(prompt).await.unwrap_err();
        assert!(matches!(err, RagError::Generation(_)));
        assert!(err.to_string().contains("500"));
    }

    #[tokio::test]
    async fn should_return_error_on_empty_choices() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "choices": []
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let adapter = GenerationAdapter::new(mock_server.uri());
        let prompt = PromptParts {
            system: "test".into(),
            context: "test".into(),
            user: "test".into(),
        };

        let err = adapter.generate(prompt).await.unwrap_err();
        assert!(matches!(err, RagError::Generation(_)));
        assert!(err.to_string().contains("empty"));
    }

    #[tokio::test]
    async fn should_include_citation_instruction_in_system_message() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "choices": [{"message": {"role": "assistant", "content": "ok"}}]
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let adapter = GenerationAdapter::new(mock_server.uri());
        let prompt = PromptParts {
            system: "Sei Gaspare Spontini.".into(),
            context: "context".into(),
            user: "question".into(),
        };

        let _ = adapter.generate(prompt).await.unwrap();

        let requests = mock_server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);

        let body: CompletionRequest = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body.messages.len(), 2);
        assert!(body.messages[0].content.contains("Cita il documento"));
        assert!(body.messages[0].content.contains("Sei Gaspare Spontini."));
        assert!(body.messages[1].content.contains("<context>"));
        assert!(body.messages[1].content.contains("<question>"));
    }
}
