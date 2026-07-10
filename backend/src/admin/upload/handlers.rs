use std::sync::Arc;

use axum::Json;
use axum::extract::{Multipart, Path, State};
use axum::http::{HeaderMap, StatusCode};
use serde::Serialize;

use crate::config::Config;

use super::UploadError;
use super::extractors::CompositeExtractor;
use super::preview_store::{PreviewEntry, UploadMetadata};

#[derive(Serialize)]
pub struct UploadResponse {
    pub token: String,
    pub preview_url: String,
}

#[derive(Serialize)]
pub struct PreviewResponse {
    pub extracted_text: String,
    pub format: String,
    pub byte_size: usize,
    pub section: String,
    pub filename: String,
    pub metadata: MetadataResponse,
    pub chunk_count_estimate: usize,
}

#[derive(Serialize)]
pub struct MetadataResponse {
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub trust_score: Option<f32>,
}

#[derive(Serialize)]
pub struct ConfirmResponse {
    pub document_ids: Vec<i64>,
    pub chunk_count: usize,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

fn check_admin_key(
    headers: &HeaderMap,
    config: &Config,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let header_val = headers
        .get("x-admin-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if header_val != config.admin_api_key {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "invalid or missing X-Admin-Key header".into(),
            }),
        ));
    }
    Ok(())
}

fn map_upload_error(e: UploadError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        UploadError::UnsupportedFormat(_) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        ),
        UploadError::FileTooLarge { .. } => (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        ),
        UploadError::PreviewNotFound => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        ),
        UploadError::InvalidRequest(_) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        ),
    }
}

pub async fn upload_document(
    State(state): State<UploadState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<UploadResponse>), (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut section: Option<String> = None;
    let mut category: Option<String> = None;
    let mut tags: Option<Vec<String>> = None;
    let mut trust_score: Option<f32> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| map_upload_error(UploadError::InvalidRequest(e.to_string())))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                filename = field.file_name().map(|s| s.to_string());
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| map_upload_error(UploadError::InvalidRequest(e.to_string())))?;
                if bytes.len() > state.config.upload_max_bytes {
                    return Err(map_upload_error(UploadError::FileTooLarge {
                        size: bytes.len(),
                        max: state.config.upload_max_bytes,
                    }));
                }
                file_bytes = Some(bytes.to_vec());
            }
            "section" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| map_upload_error(UploadError::InvalidRequest(e.to_string())))?;
                section = Some(text);
            }
            "category" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| map_upload_error(UploadError::InvalidRequest(e.to_string())))?;
                if !text.is_empty() {
                    category = Some(text);
                }
            }
            "tags" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| map_upload_error(UploadError::InvalidRequest(e.to_string())))?;
                if !text.is_empty() {
                    tags = Some(text.split(',').map(|s| s.trim().to_string()).collect());
                }
            }
            "trust_score" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| map_upload_error(UploadError::InvalidRequest(e.to_string())))?;
                if !text.is_empty() {
                    trust_score = text.parse().ok();
                }
            }
            _ => {}
        }
    }

    let file_bytes = file_bytes.ok_or_else(|| {
        map_upload_error(UploadError::InvalidRequest("missing file field".into()))
    })?;
    let filename = filename
        .ok_or_else(|| map_upload_error(UploadError::InvalidRequest("missing filename".into())))?;
    let section = section.ok_or_else(|| {
        map_upload_error(UploadError::InvalidRequest("missing section field".into()))
    })?;

    let extracted =
        CompositeExtractor::extract(&file_bytes, &filename).map_err(map_upload_error)?;

    let entry = PreviewEntry {
        extracted_text: extracted,
        section,
        metadata: UploadMetadata {
            category,
            tags,
            trust_score,
        },
        filename,
        created_at: chrono::Utc::now(),
    };

    let token = state.preview_store.insert(entry);
    let preview_url = format!("/admin/api/upload/preview/{}", token);

    Ok((
        StatusCode::CREATED,
        Json(UploadResponse { token, preview_url }),
    ))
}

pub async fn get_preview(
    State(state): State<UploadState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<Json<PreviewResponse>, (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

    let entry = state.preview_store.get(&token).map_err(map_upload_error)?;

    let chunk_count_estimate = (entry.extracted_text.content.len() / 512).max(1);

    Ok(Json(PreviewResponse {
        extracted_text: entry.extracted_text.content,
        format: entry.extracted_text.format.to_string(),
        byte_size: entry.extracted_text.byte_size,
        section: entry.section,
        filename: entry.filename,
        metadata: MetadataResponse {
            category: entry.metadata.category,
            tags: entry.metadata.tags,
            trust_score: entry.metadata.trust_score,
        },
        chunk_count_estimate,
    }))
}

pub async fn confirm_upload(
    State(state): State<UploadState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<(StatusCode, Json<ConfirmResponse>), (StatusCode, Json<ErrorResponse>)> {
    check_admin_key(&headers, &state.config)?;

    let entry = state
        .preview_store
        .remove(&token)
        .map_err(map_upload_error)?;

    let document_ids = state
        .upload
        .ingest_uploaded(
            &entry.extracted_text.content,
            &entry.section,
            &entry.filename,
            &entry.metadata,
        )
        .await
        .map_err(map_upload_error)?;

    let chunk_count = document_ids.len();

    Ok((
        StatusCode::OK,
        Json(ConfirmResponse {
            document_ids,
            chunk_count,
        }),
    ))
}

#[derive(Clone)]
pub struct UploadState {
    pub upload: Arc<dyn super::ports::UploadPort>,
    pub preview_store: Arc<super::preview_store::PreviewStore>,
    pub config: Config,
}
