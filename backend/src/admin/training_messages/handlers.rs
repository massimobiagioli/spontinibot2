use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use serde::Deserialize;

use crate::admin::ErrorResponse;
use crate::admin::check_admin_key;
use crate::admin::training_messages::{
    TrainingMessageAdminPort, TrainingMessageError, TrainingMessageResponse,
};
use crate::config::Config;

#[derive(Clone)]
pub struct TrainingMessageState {
    pub training_messages: Arc<dyn TrainingMessageAdminPort>,
    pub config: Config,
}

#[derive(Deserialize)]
pub struct AskRequest {
    pub question: String,
}

fn map_message_error(e: TrainingMessageError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        TrainingMessageError::SessionNotFound(id) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("training session {id} not found"),
            }),
        ),
        TrainingMessageError::DbError(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: msg }),
        ),
        TrainingMessageError::Rag(msg) => {
            (StatusCode::BAD_GATEWAY, Json(ErrorResponse { error: msg }))
        }
    }
}

pub async fn create_message(
    State(state): State<TrainingMessageState>,
    headers: HeaderMap,
    Path(session_id): Path<i64>,
    Json(req): Json<AskRequest>,
) -> Result<(StatusCode, Json<TrainingMessageResponse>), (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

    let response = state
        .training_messages
        .ask(session_id, req.question)
        .await
        .map_err(map_message_error)?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list_messages(
    State(state): State<TrainingMessageState>,
    headers: HeaderMap,
    Path(session_id): Path<i64>,
) -> Result<Json<Vec<TrainingMessageResponse>>, (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

    let messages = state
        .training_messages
        .list_messages(session_id)
        .await
        .map_err(map_message_error)?;
    Ok(Json(messages))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::training_messages::TrainingMessageSource;

    fn test_state() -> TrainingMessageState {
        let config = Config {
            embed_url: "http://localhost:8080".into(),
            generate_url: "http://localhost:8081".into(),
            kb_path: "/tmp/test.db".into(),
            top_k: 5,
            min_score: 0.35,
            admin_api_key: "test-key".into(),
            upload_max_bytes: 10_485_760,
        };
        TrainingMessageState {
            training_messages: Arc::new(MockTrainingMessageAdmin),
            config,
        }
    }

    struct MockTrainingMessageAdmin;

    #[async_trait::async_trait]
    impl TrainingMessageAdminPort for MockTrainingMessageAdmin {
        async fn ask(
            &self,
            session_id: i64,
            question: String,
        ) -> Result<TrainingMessageResponse, TrainingMessageError> {
            if session_id == 999 {
                return Err(TrainingMessageError::SessionNotFound(session_id));
            }
            Ok(TrainingMessageResponse {
                id: 1,
                session_id,
                question,
                answer: "Lo sportello apre alle 9:00.".into(),
                sources: vec![TrainingMessageSource {
                    document_id: 7,
                    source_ref: "orari.md".into(),
                }],
                fell_back: false,
                created_at: "2026-07-24T00:00:00Z".into(),
            })
        }

        async fn list_messages(
            &self,
            session_id: i64,
        ) -> Result<Vec<TrainingMessageResponse>, TrainingMessageError> {
            Ok(vec![TrainingMessageResponse {
                id: 1,
                session_id,
                question: "A che ora apre l'anagrafe?".into(),
                answer: "Lo sportello apre alle 9:00.".into(),
                sources: vec![],
                fell_back: false,
                created_at: "2026-07-24T00:00:00Z".into(),
            }])
        }
    }

    fn auth_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-admin-key", "test-key".parse().unwrap());
        headers
    }

    fn no_auth_headers() -> HeaderMap {
        HeaderMap::new()
    }

    #[tokio::test]
    async fn should_reject_create_message_without_admin_key() {
        let state = test_state();
        let req = AskRequest {
            question: "domanda".into(),
        };
        let result = create_message(State(state), no_auth_headers(), Path(1), Json(req)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn should_create_message_and_return_created() {
        let state = test_state();
        let req = AskRequest {
            question: "A che ora apre l'anagrafe?".into(),
        };
        let result = create_message(State(state), auth_headers(), Path(1), Json(req)).await;
        assert!(result.is_ok());
        let (status, Json(response)) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(response.session_id, 1);
        assert_eq!(response.sources.len(), 1);
    }

    #[tokio::test]
    async fn should_return_404_for_unknown_session_on_create() {
        let state = test_state();
        let req = AskRequest {
            question: "domanda".into(),
        };
        let result = create_message(State(state), auth_headers(), Path(999), Json(req)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn should_reject_list_messages_without_admin_key() {
        let state = test_state();
        let result = list_messages(State(state), no_auth_headers(), Path(1)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_list_messages_for_known_session() {
        let state = test_state();
        let result = list_messages(State(state), auth_headers(), Path(1)).await;
        assert!(result.is_ok());
        let Json(messages) = result.unwrap();
        assert_eq!(messages.len(), 1);
    }
}
