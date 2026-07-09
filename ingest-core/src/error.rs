use thiserror::Error;

#[derive(Error, Debug)]
pub enum IngestError {
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("HTTP error response: {status} {body}")]
    HttpStatus { status: u16, body: String },

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("robots.txt: {0}")]
    RobotsTxt(String),

    #[error("disallowed content-type: {0}")]
    ContentType(String),

    #[error("chunking error: {0}")]
    Chunking(String),

    #[error("embedding error: {0}")]
    Embedding(String),

    #[error("dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("KB store error: {0}")]
    KbStore(#[from] kb_store::KbStoreError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_display_content_type_error_message() {
        let err = IngestError::ContentType("application/pdf".into());
        assert_eq!(err.to_string(), "disallowed content-type: application/pdf");
    }

    #[test]
    fn should_display_http_status_error_message() {
        let err = IngestError::HttpStatus {
            status: 500,
            body: "internal error".into(),
        };
        assert_eq!(err.to_string(), "HTTP error response: 500 internal error");
    }

    #[test]
    fn should_display_robots_txt_error_message() {
        let err = IngestError::RobotsTxt("/admin/ is disallowed".into());
        assert_eq!(err.to_string(), "robots.txt: /admin/ is disallowed");
    }

    #[test]
    fn should_display_content_type_error_with_unknown_type() {
        let err = IngestError::ContentType("image/png".into());
        assert_eq!(err.to_string(), "disallowed content-type: image/png");
    }

    #[test]
    fn should_display_chunking_error_message() {
        let err = IngestError::Chunking("empty input".into());
        assert_eq!(err.to_string(), "chunking error: empty input");
    }

    #[test]
    fn should_display_embedding_error_message() {
        let err = IngestError::Embedding("server refused".into());
        assert_eq!(err.to_string(), "embedding error: server refused");
    }

    #[test]
    fn should_display_dimension_mismatch() {
        let err = IngestError::DimensionMismatch {
            expected: 768,
            actual: 512,
        };
        assert_eq!(err.to_string(), "dimension mismatch: expected 768, got 512");
    }
}
