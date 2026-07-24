use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use serde::Deserialize;

use crate::admin::ErrorResponse;
use crate::admin::check_admin_key;
use crate::admin::training_feedback::{
    TrainingFeedbackAdminPort, TrainingFeedbackError, TrainingFeedbackResponse,
};
use crate::config::Config;

#[derive(Clone)]
pub struct TrainingFeedbackState {
    pub training_feedback: Arc<dyn TrainingFeedbackAdminPort>,
    pub config: Config,
}

#[derive(Deserialize)]
pub struct CreateFeedbackRequest {
    pub message_id: i64,
    pub chunk_id: Option<i64>,
    pub answer_span: String,
    pub sentiment: String,
    pub comment: Option<String>,
}

fn map_feedback_error(e: TrainingFeedbackError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        TrainingFeedbackError::MessageNotFound(id) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("training message {id} not found"),
            }),
        ),
        TrainingFeedbackError::DbError(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: msg }),
        ),
    }
}

pub async fn create_feedback(
    State(state): State<TrainingFeedbackState>,
    headers: HeaderMap,
    Json(req): Json<CreateFeedbackRequest>,
) -> Result<(StatusCode, Json<TrainingFeedbackResponse>), (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

    let sentiment: kb_store::Sentiment = req.sentiment.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("invalid sentiment: {}", req.sentiment),
            }),
        )
    })?;

    let feedback = kb_store::NewTrainingFeedback {
        message_id: req.message_id,
        chunk_id: req.chunk_id,
        answer_span: req.answer_span,
        sentiment,
        comment: req.comment,
    };
    let response = state
        .training_feedback
        .create_feedback(feedback)
        .await
        .map_err(map_feedback_error)?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list_feedback(
    State(state): State<TrainingFeedbackState>,
    headers: HeaderMap,
    Path(message_id): Path<i64>,
) -> Result<Json<Vec<TrainingFeedbackResponse>>, (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

    let feedback = state
        .training_feedback
        .list_feedback(message_id)
        .await
        .map_err(map_feedback_error)?;
    Ok(Json(feedback))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> TrainingFeedbackState {
        let config = Config {
            embed_url: "http://localhost:8080".into(),
            generate_url: "http://localhost:8081".into(),
            kb_path: "/tmp/test.db".into(),
            top_k: 5,
            min_score: 0.35,
            admin_api_key: "test-key".into(),
            upload_max_bytes: 10_485_760,
        };
        TrainingFeedbackState {
            training_feedback: Arc::new(MockTrainingFeedbackAdmin),
            config,
        }
    }

    struct MockTrainingFeedbackAdmin;

    #[async_trait::async_trait]
    impl TrainingFeedbackAdminPort for MockTrainingFeedbackAdmin {
        async fn create_feedback(
            &self,
            req: kb_store::NewTrainingFeedback,
        ) -> Result<TrainingFeedbackResponse, TrainingFeedbackError> {
            if req.message_id == 999 {
                return Err(TrainingFeedbackError::MessageNotFound(req.message_id));
            }
            if req.message_id == 500 {
                return Err(TrainingFeedbackError::DbError("connection refused".into()));
            }
            Ok(TrainingFeedbackResponse {
                id: 1,
                message_id: req.message_id,
                chunk_id: req.chunk_id,
                answer_span: req.answer_span,
                sentiment: req.sentiment.to_string(),
                comment: req.comment,
                created_at: "2026-07-24T00:00:00Z".into(),
            })
        }

        async fn list_feedback(
            &self,
            message_id: i64,
        ) -> Result<Vec<TrainingFeedbackResponse>, TrainingFeedbackError> {
            Ok(vec![TrainingFeedbackResponse {
                id: 1,
                message_id,
                chunk_id: None,
                answer_span: "alle 9:00".into(),
                sentiment: "positive".into(),
                comment: None,
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

    fn sample_request() -> CreateFeedbackRequest {
        CreateFeedbackRequest {
            message_id: 1,
            chunk_id: Some(7),
            answer_span: "alle 9:00".into(),
            sentiment: "positive".into(),
            comment: None,
        }
    }

    #[tokio::test]
    async fn should_reject_create_feedback_without_admin_key() {
        let state = test_state();
        let result = create_feedback(State(state), no_auth_headers(), Json(sample_request())).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn should_create_feedback_and_return_created() {
        let state = test_state();
        let result = create_feedback(State(state), auth_headers(), Json(sample_request())).await;
        assert!(result.is_ok());
        let (status, Json(response)) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(response.sentiment, "positive");
        assert_eq!(response.chunk_id, Some(7));
    }

    #[tokio::test]
    async fn should_return_404_for_unknown_message_on_create() {
        let state = test_state();
        let mut req = sample_request();
        req.message_id = 999;
        let result = create_feedback(State(state), auth_headers(), Json(req)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn should_return_400_for_invalid_sentiment() {
        let state = test_state();
        let mut req = sample_request();
        req.sentiment = "neutral".into();
        let result = create_feedback(State(state), auth_headers(), Json(req)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn should_return_500_when_create_feedback_hits_a_db_error() {
        let state = test_state();
        let mut req = sample_request();
        req.message_id = 500;
        let result = create_feedback(State(state), auth_headers(), Json(req)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn should_reject_list_feedback_without_admin_key() {
        let state = test_state();
        let result = list_feedback(State(state), no_auth_headers(), Path(1)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_list_feedback_for_known_message() {
        let state = test_state();
        let result = list_feedback(State(state), auth_headers(), Path(1)).await;
        assert!(result.is_ok());
        let Json(feedback) = result.unwrap();
        assert_eq!(feedback.len(), 1);
    }
}
