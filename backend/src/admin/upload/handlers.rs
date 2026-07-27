use std::sync::Arc;

use axum::Json;
use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use serde::Serialize;

use crate::audit::AuditLogPort;
use crate::audit::record_best_effort;
use crate::auth::extractor::OperatorSession;
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
    _session: OperatorSession,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<UploadResponse>), (StatusCode, Json<ErrorResponse>)> {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut section: Option<String> = None;

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

    // The operator uploads into a section already, and manually tuning a
    // trust score or picking tags is busywork for them — derive all three
    // automatically instead: category mirrors the section, trust_score
    // reflects that an operator curated this upload, and tags come from the
    // extracted text's own significant words (see `tagging::extract_tags`).
    let category = Some(section.clone());
    let trust_score = Some(0.9);
    let derived_tags = super::tagging::extract_tags(&extracted.content, 5);
    let tags = if derived_tags.is_empty() {
        None
    } else {
        Some(derived_tags)
    };

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
    _session: OperatorSession,
    Path(token): Path<String>,
) -> Result<Json<PreviewResponse>, (StatusCode, Json<ErrorResponse>)> {
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
    session: OperatorSession,
    Path(token): Path<String>,
) -> Result<(StatusCode, Json<ConfirmResponse>), (StatusCode, Json<ErrorResponse>)> {
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

    record_best_effort(
        state.audit.as_ref(),
        &session.actor,
        "confirm_upload",
        &format!("upload:{}", entry.filename),
        &serde_json::json!({"document_ids": document_ids, "chunk_count": chunk_count}),
    )
    .await;

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
    pub audit: Arc<dyn AuditLogPort>,
}
