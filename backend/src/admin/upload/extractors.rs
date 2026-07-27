//! Text extraction adapters for various document formats.
//!
//! Each extractor implements format-specific text extraction logic. The
//! `CompositeExtractor` dispatches to the appropriate extractor based on
//! the file extension.

use super::{DocumentFormat, ExtractedText, UploadError};

/// Extract text from PDF bytes using the `pdf-extract` crate.
pub struct PdfExtractor;

impl PdfExtractor {
    pub fn extract(file_bytes: &[u8]) -> Result<ExtractedText, UploadError> {
        // Validate PDF magic bytes
        if file_bytes.len() < 5 || &file_bytes[..5] != b"%PDF-" {
            return Err(UploadError::ExtractionFailed(
                "invalid PDF: missing magic bytes".into(),
            ));
        }

        let content = pdf_extract::extract_text_from_mem(file_bytes)
            .map_err(|e| UploadError::ExtractionFailed(format!("PDF extraction failed: {e}")))?;

        Ok(ExtractedText {
            content,
            format: DocumentFormat::Pdf,
            byte_size: file_bytes.len(),
        })
    }
}

/// Extract text from DOCX bytes using the `docx-rs` crate.
pub struct DocxExtractor;

impl DocxExtractor {
    pub fn extract(file_bytes: &[u8]) -> Result<ExtractedText, UploadError> {
        // Validate DOCX magic bytes (ZIP signature)
        if file_bytes.len() < 4 || &file_bytes[..4] != b"PK\x03\x04" {
            return Err(UploadError::ExtractionFailed(
                "invalid DOCX: missing ZIP signature".into(),
            ));
        }

        let docx = docx_rs::read_docx(file_bytes)
            .map_err(|e| UploadError::ExtractionFailed(format!("DOCX parsing failed: {e}")))?;

        let mut content = String::new();
        for paragraph in docx.document.children {
            if let docx_rs::DocumentChild::Paragraph(p) = paragraph {
                for child in p.children {
                    if let docx_rs::ParagraphChild::Run(r) = child {
                        for run_child in r.children {
                            if let docx_rs::RunChild::Text(t) = run_child {
                                content.push_str(&t.text);
                            }
                        }
                    }
                }
                content.push('\n');
            }
        }

        Ok(ExtractedText {
            content,
            format: DocumentFormat::Docx,
            byte_size: file_bytes.len(),
        })
    }
}

/// Extract text from RTF bytes using the `rtf-parser` crate.
pub struct RtfExtractor;

impl RtfExtractor {
    pub fn extract(file_bytes: &[u8]) -> Result<ExtractedText, UploadError> {
        if file_bytes.len() < 5 || &file_bytes[..5] != b"{\\rtf" {
            return Err(UploadError::ExtractionFailed(
                "invalid RTF: missing '{\\rtf' signature".into(),
            ));
        }

        let rtf_str = std::str::from_utf8(file_bytes)
            .map_err(|e| UploadError::ExtractionFailed(format!("invalid UTF-8 in RTF: {e}")))?;

        let tokens = rtf_parser::lexer::Lexer::scan(rtf_str)
            .map_err(|e| UploadError::ExtractionFailed(format!("RTF lexing failed: {e}")))?;
        let mut parser = rtf_parser::parser::Parser::new(tokens);
        let document = parser
            .parse()
            .map_err(|e| UploadError::ExtractionFailed(format!("RTF parsing failed: {e}")))?;
        let content = document.get_text();

        Ok(ExtractedText {
            content,
            format: DocumentFormat::Rtf,
            byte_size: file_bytes.len(),
        })
    }
}

/// Extract text from Markdown bytes (UTF-8 read, strip optional frontmatter).
pub struct MarkdownExtractor;

impl MarkdownExtractor {
    pub fn extract(file_bytes: &[u8]) -> Result<ExtractedText, UploadError> {
        let content = std::str::from_utf8(file_bytes)
            .map_err(|e| UploadError::ExtractionFailed(format!("invalid UTF-8: {e}")))?
            .to_string();

        // Strip optional YAML frontmatter (--- ... ---)
        let content = if let Some(after_first) = content.strip_prefix("---") {
            if let Some(end) = after_first.find("---") {
                after_first[end + 3..].trim_start().to_string()
            } else {
                content
            }
        } else {
            content
        };

        Ok(ExtractedText {
            content,
            format: DocumentFormat::Markdown,
            byte_size: file_bytes.len(),
        })
    }
}

/// Extract text from plain text bytes (UTF-8 read verbatim).
pub struct PlainTextExtractor;

impl PlainTextExtractor {
    pub fn extract(file_bytes: &[u8]) -> Result<ExtractedText, UploadError> {
        let content = std::str::from_utf8(file_bytes)
            .map_err(|e| UploadError::ExtractionFailed(format!("invalid UTF-8: {e}")))?
            .to_string();

        Ok(ExtractedText {
            content,
            format: DocumentFormat::PlainText,
            byte_size: file_bytes.len(),
        })
    }
}

/// Dispatches to the appropriate extractor based on file extension.
pub struct CompositeExtractor;

impl CompositeExtractor {
    pub fn extract(file_bytes: &[u8], filename: &str) -> Result<ExtractedText, UploadError> {
        let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();

        match ext.as_str() {
            "pdf" => PdfExtractor::extract(file_bytes),
            "docx" => DocxExtractor::extract(file_bytes),
            "rtf" => RtfExtractor::extract(file_bytes),
            "md" | "markdown" => MarkdownExtractor::extract(file_bytes),
            "txt" | "text" => PlainTextExtractor::extract(file_bytes),
            _ => Err(UploadError::UnsupportedFormat(ext)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_extract_plain_text() {
        let bytes = b"Hello, world!";
        let result = PlainTextExtractor::extract(bytes).unwrap();
        assert_eq!(result.content, "Hello, world!");
        assert_eq!(result.format, DocumentFormat::PlainText);
        assert_eq!(result.byte_size, 13);
    }

    #[test]
    fn should_extract_markdown_without_frontmatter() {
        let bytes = b"# Title\n\nSome content.";
        let result = MarkdownExtractor::extract(bytes).unwrap();
        assert_eq!(result.content, "# Title\n\nSome content.");
        assert_eq!(result.format, DocumentFormat::Markdown);
    }

    #[test]
    fn should_strip_markdown_frontmatter() {
        let bytes = b"---\ntitle: Test\n---\n\n# Title\n\nContent.";
        let result = MarkdownExtractor::extract(bytes).unwrap();
        assert!(result.content.starts_with("# Title"));
        assert!(!result.content.contains("title: Test"));
    }

    #[test]
    fn should_reject_invalid_utf8_for_plain_text() {
        let bytes = &[0xFF, 0xFE, 0x00];
        let result = PlainTextExtractor::extract(bytes);
        assert!(matches!(result, Err(UploadError::ExtractionFailed(_))));
    }

    #[test]
    fn should_reject_invalid_pdf_magic_bytes() {
        let bytes = b"not a pdf";
        let result = PdfExtractor::extract(bytes);
        assert!(matches!(result, Err(UploadError::ExtractionFailed(_))));
    }

    #[test]
    fn should_reject_invalid_docx_magic_bytes() {
        let bytes = b"not a docx";
        let result = DocxExtractor::extract(bytes);
        assert!(matches!(result, Err(UploadError::ExtractionFailed(_))));
    }

    #[test]
    fn should_extract_plain_text_from_rtf() {
        let rtf = br#"{\rtf1\ansi{\fonttbl\f0\fswiss Helvetica;}\f0\pard Voici du texte en {\b gras}.\par}"#;
        let result = RtfExtractor::extract(rtf).unwrap();
        assert_eq!(result.content, "Voici du texte en gras.");
        assert_eq!(result.format, DocumentFormat::Rtf);
    }

    #[test]
    fn should_reject_invalid_rtf_magic_bytes() {
        let bytes = b"not an rtf document";
        let result = RtfExtractor::extract(bytes);
        assert!(matches!(result, Err(UploadError::ExtractionFailed(_))));
    }

    #[test]
    fn should_dispatch_by_extension() {
        let txt_bytes = b"plain text";
        let result = CompositeExtractor::extract(txt_bytes, "file.txt").unwrap();
        assert_eq!(result.format, DocumentFormat::PlainText);

        let md_bytes = b"# Markdown";
        let result = CompositeExtractor::extract(md_bytes, "file.md").unwrap();
        assert_eq!(result.format, DocumentFormat::Markdown);

        let rtf_bytes = br#"{\rtf1\ansi Hello.\par}"#;
        let result = CompositeExtractor::extract(rtf_bytes, "file.rtf").unwrap();
        assert_eq!(result.format, DocumentFormat::Rtf);
    }

    #[test]
    fn should_return_unsupported_format_for_unknown_extension() {
        let bytes = b"some bytes";
        let result = CompositeExtractor::extract(bytes, "file.jpg");
        assert!(matches!(result, Err(UploadError::UnsupportedFormat(_))));
    }

    #[test]
    fn should_handle_case_insensitive_extensions() {
        let bytes = b"text content";
        let result = CompositeExtractor::extract(bytes, "file.TXT").unwrap();
        assert_eq!(result.format, DocumentFormat::PlainText);
    }
}
