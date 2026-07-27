//! Manual document upload surface for the admin API.
//!
//! This module implements the two-step upload flow: upload → preview → confirm.
//! The preview step extracts text from the uploaded file and stores it in memory
//! with a short-lived token. The confirm step delegates to `ingest-core` for
//! chunking, embedding, and insertion into `kb.db`.

pub mod adapter;
pub mod extractors;
pub mod handlers;
pub mod ports;
pub mod preview_store;
pub mod tagging;

use std::fmt;

/// The format of an uploaded document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFormat {
    Pdf,
    Docx,
    Rtf,
    Markdown,
    PlainText,
}

impl fmt::Display for DocumentFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocumentFormat::Pdf => write!(f, "pdf"),
            DocumentFormat::Docx => write!(f, "docx"),
            DocumentFormat::Rtf => write!(f, "rtf"),
            DocumentFormat::Markdown => write!(f, "markdown"),
            DocumentFormat::PlainText => write!(f, "plain_text"),
        }
    }
}

/// The result of text extraction from an uploaded file.
#[derive(Debug, Clone)]
pub struct ExtractedText {
    pub content: String,
    pub format: DocumentFormat,
    pub byte_size: usize,
}

/// Errors that can occur during the upload flow.
#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    #[error("unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("file too large: {size} bytes (max: {max})")]
    FileTooLarge { size: usize, max: usize },

    #[error("preview token not found or expired")]
    PreviewNotFound,

    #[error("ingest pipeline error: {0}")]
    IngestFailed(String),

    #[error("invalid multipart request: {0}")]
    InvalidRequest(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_display_document_format() {
        assert_eq!(DocumentFormat::Pdf.to_string(), "pdf");
        assert_eq!(DocumentFormat::Docx.to_string(), "docx");
        assert_eq!(DocumentFormat::Markdown.to_string(), "markdown");
        assert_eq!(DocumentFormat::PlainText.to_string(), "plain_text");
    }

    #[test]
    fn should_display_upload_error() {
        let err = UploadError::UnsupportedFormat("jpg".into());
        assert!(err.to_string().contains("jpg"));

        let err = UploadError::FileTooLarge {
            size: 20_000_000,
            max: 10_000_000,
        };
        assert!(err.to_string().contains("20000000"));
    }

    #[test]
    fn should_construct_extracted_text() {
        let extracted = ExtractedText {
            content: "Hello world".into(),
            format: DocumentFormat::PlainText,
            byte_size: 11,
        };
        assert_eq!(extracted.content, "Hello world");
        assert_eq!(extracted.byte_size, 11);
    }
}
