pub mod adapter;
pub mod handlers;

use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use kb_store::{
    IngestBookmark, IngestSchedule, IngestSection, IngestSource, IngestedDocument,
    NewIngestSchedule, SourceType,
};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct IngestScheduleResponse {
    pub cron_expr: String,
    pub enabled: bool,
    pub updated_at: String,
}

impl From<IngestSchedule> for IngestScheduleResponse {
    fn from(s: IngestSchedule) -> Self {
        Self {
            cron_expr: s.cron_expr,
            enabled: s.enabled,
            updated_at: s.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct IngestSectionResponse {
    pub id: i64,
    pub name: String,
    pub ordering: i32,
    pub created_at: String,
}

impl From<IngestSection> for IngestSectionResponse {
    fn from(s: IngestSection) -> Self {
        Self {
            id: s.id,
            name: s.name,
            ordering: s.ordering,
            created_at: s.created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct IngestSourceResponse {
    pub id: i64,
    pub section_id: i64,
    pub source_type: String,
    pub url: String,
    pub enabled: bool,
    pub created_at: String,
    pub coming_soon: bool,
}

impl From<IngestSource> for IngestSourceResponse {
    fn from(s: IngestSource) -> Self {
        let is_api = s.source_type == SourceType::Api;
        Self {
            id: s.id,
            section_id: s.section_id,
            source_type: s.source_type.to_string(),
            url: s.url,
            enabled: if is_api { false } else { s.enabled },
            created_at: s.created_at,
            coming_soon: is_api,
        }
    }
}

/// A non-interactive curation source (Plan 0030) — the durable checkpoint
/// `ingest_bookmark` records for a section's automated curation, as opposed
/// to an `IngestSourceResponse` (a recurring, `robots.txt`-honoring scrape/
/// api source the scheduler actively polls). Distinct on purpose: a curation
/// source is a byproduct of the curation adapter running, not an operator-
/// editable "Fonti" entry — it has no `enabled`/delete affordance.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CurationSourceResponse {
    pub source_url: String,
    pub last_item_date: String,
    pub updated_at: String,
}

impl From<IngestBookmark> for CurationSourceResponse {
    fn from(b: IngestBookmark) -> Self {
        Self {
            source_url: b.source_url,
            last_item_date: b.last_item_date,
            updated_at: b.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct IngestSectionWithSources {
    #[serde(flatten)]
    pub section: IngestSectionResponse,
    pub sources: Vec<IngestSourceResponse>,
    pub curation_sources: Vec<CurationSourceResponse>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct IngestConfigResponse {
    pub schedule: Option<IngestScheduleResponse>,
    pub sections: Vec<IngestSectionWithSources>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct IngestedDocumentResponse {
    pub source_ref: String,
    pub source: String,
    pub chunk_count: i64,
    pub created_at: String,
    pub summary: Option<String>,
}

impl From<IngestedDocument> for IngestedDocumentResponse {
    fn from(d: IngestedDocument) -> Self {
        Self {
            source_ref: d.source_ref,
            source: d.source.to_string(),
            chunk_count: d.chunk_count,
            created_at: d.created_at,
            summary: d.summary,
        }
    }
}

#[derive(Debug)]
pub enum IngestConfigError {
    NotFound(String),
    DbError(String),
}

impl fmt::Display for IngestConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IngestConfigError::NotFound(msg) => write!(f, "not found: {msg}"),
            IngestConfigError::DbError(msg) => write!(f, "database error: {msg}"),
        }
    }
}

impl std::error::Error for IngestConfigError {}

impl From<kb_store::KbStoreError> for IngestConfigError {
    fn from(e: kb_store::KbStoreError) -> Self {
        IngestConfigError::DbError(e.to_string())
    }
}

#[async_trait]
pub trait IngestConfigAdminPort: Send + Sync {
    async fn get_schedule(&self) -> Result<Option<IngestScheduleResponse>, IngestConfigError>;
    async fn upsert_schedule(
        &self,
        schedule: NewIngestSchedule,
    ) -> Result<IngestScheduleResponse, IngestConfigError>;
    async fn list_sections(&self) -> Result<Vec<IngestSectionResponse>, IngestConfigError>;
    async fn create_section(
        &self,
        section: kb_store::NewIngestSection,
    ) -> Result<IngestSectionResponse, IngestConfigError>;
    async fn delete_section(&self, id: i64) -> Result<bool, IngestConfigError>;
    async fn list_sources(
        &self,
        section_id: i64,
    ) -> Result<Vec<IngestSourceResponse>, IngestConfigError>;
    async fn list_curation_sources(
        &self,
        section_id: i64,
    ) -> Result<Vec<CurationSourceResponse>, IngestConfigError>;
    async fn create_source(
        &self,
        section_id: i64,
        source: kb_store::NewIngestSource,
    ) -> Result<IngestSourceResponse, IngestConfigError>;
    async fn delete_source(&self, id: i64) -> Result<bool, IngestConfigError>;
    async fn list_section_documents(
        &self,
        section_id: i64,
    ) -> Result<Vec<IngestedDocumentResponse>, IngestConfigError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_transform_api_source_to_coming_soon() {
        let source = IngestSource {
            id: 1,
            section_id: 10,
            source_type: SourceType::Api,
            url: "https://api.example.com".into(),
            enabled: true,
            created_at: "2026-07-10T00:00:00Z".into(),
        };
        let response = IngestSourceResponse::from(source);
        assert!(!response.enabled);
        assert!(response.coming_soon);
        assert_eq!(response.source_type, "api");
    }

    #[test]
    fn should_preserve_scrape_source_enabled_state() {
        let source = IngestSource {
            id: 2,
            section_id: 10,
            source_type: SourceType::Scrape,
            url: "https://example.com".into(),
            enabled: true,
            created_at: "2026-07-10T00:00:00Z".into(),
        };
        let response = IngestSourceResponse::from(source);
        assert!(response.enabled);
        assert!(!response.coming_soon);
        assert_eq!(response.source_type, "scrape");
    }

    #[test]
    fn should_format_ingest_config_error_display() {
        let not_found = IngestConfigError::NotFound("section 5".into());
        assert_eq!(not_found.to_string(), "not found: section 5");

        let db_error = IngestConfigError::DbError("connection refused".into());
        assert_eq!(db_error.to_string(), "database error: connection refused");
    }

    #[test]
    fn should_convert_schedule_to_response() {
        let schedule = IngestSchedule {
            cron_expr: "0 */4 * * *".into(),
            enabled: true,
            updated_at: "2026-07-10T00:00:00Z".into(),
        };
        let response = IngestScheduleResponse::from(schedule);
        assert_eq!(response.cron_expr, "0 */4 * * *");
        assert!(response.enabled);
    }

    #[test]
    fn should_convert_ingested_document_to_response() {
        let doc = IngestedDocument {
            source_ref: "https://example.com/news/1".into(),
            source: kb_store::DocumentSource::Scrape,
            chunk_count: 3,
            created_at: "2026-07-24 00:00:00".into(),
            summary: None,
        };
        let response = IngestedDocumentResponse::from(doc);
        assert_eq!(response.source_ref, "https://example.com/news/1");
        assert_eq!(response.source, "scrape");
        assert_eq!(response.created_at, "2026-07-24 00:00:00");
        assert_eq!(response.chunk_count, 3);
        assert_eq!(response.summary, None);
    }

    #[test]
    fn should_carry_summary_through_to_the_response_when_present() {
        let doc = IngestedDocument {
            source_ref: "delibera-di-giunta-74-2026-07-13.pdf".into(),
            source: kb_store::DocumentSource::Manual,
            chunk_count: 5,
            created_at: "2026-07-27 00:00:00".into(),
            summary: Some("POSTEGGI AREA FIERA SANT'ANNA".into()),
        };
        let response = IngestedDocumentResponse::from(doc);
        assert_eq!(
            response.summary.as_deref(),
            Some("POSTEGGI AREA FIERA SANT'ANNA")
        );
    }

    #[test]
    fn should_convert_bookmark_to_curation_source_response() {
        let bookmark = IngestBookmark {
            id: 1,
            section_id: 3,
            source_url: "https://www.halleyweb.com/.../delibere".into(),
            last_item_ref: "74".into(),
            last_item_date: "2026-07-13".into(),
            updated_at: "2026-07-27 15:07:53".into(),
        };
        let response = CurationSourceResponse::from(bookmark);
        assert_eq!(
            response.source_url,
            "https://www.halleyweb.com/.../delibere"
        );
        assert_eq!(response.last_item_date, "2026-07-13");
        assert_eq!(response.updated_at, "2026-07-27 15:07:53");
    }

    #[test]
    fn should_convert_section_to_response() {
        let section = IngestSection {
            id: 3,
            name: "sport".into(),
            ordering: 10,
            created_at: "2026-07-10T00:00:00Z".into(),
        };
        let response = IngestSectionResponse::from(section);
        assert_eq!(response.id, 3);
        assert_eq!(response.name, "sport");
    }
}
