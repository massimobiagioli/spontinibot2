use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub enum DocumentSource {
    Scrape,
    Api,
    Manual,
}

impl fmt::Display for DocumentSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocumentSource::Scrape => write!(f, "scrape"),
            DocumentSource::Api => write!(f, "api"),
            DocumentSource::Manual => write!(f, "manual"),
        }
    }
}

impl FromStr for DocumentSource {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "scrape" => Ok(DocumentSource::Scrape),
            "api" => Ok(DocumentSource::Api),
            "manual" => Ok(DocumentSource::Manual),
            other => Err(format!("unknown document source: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub id: i64,
    pub source: DocumentSource,
    pub source_ref: String,
    pub content: String,
    pub metadata: Option<String>,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug)]
pub struct NewDocument {
    pub source: DocumentSource,
    pub source_ref: String,
    pub content: String,
    pub metadata: Option<String>,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct ScoredDocument {
    pub document: Document,
    pub similarity: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Persona {
    pub id: i64,
    pub version: i32,
    pub name: String,
    pub system_prompt: String,
    pub tone: Option<String>,
    pub fallback_message: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub created_by: Option<String>,
}

#[derive(Debug)]
pub struct NewPersona {
    pub name: String,
    pub system_prompt: String,
    pub tone: Option<String>,
    pub fallback_message: Option<String>,
    pub created_by: Option<String>,
}

pub const EMBEDDING_DIM: usize = 768;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_roundtrip_document_source_via_display_and_fromstr() {
        let cases = [
            (DocumentSource::Scrape, "scrape"),
            (DocumentSource::Api, "api"),
            (DocumentSource::Manual, "manual"),
        ];
        for (variant, expected) in cases {
            assert_eq!(variant.to_string(), expected);
            let parsed: DocumentSource = expected.parse().unwrap();
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn should_reject_unknown_document_source() {
        let result: std::result::Result<DocumentSource, _> = "unknown".parse();
        assert!(result.is_err());
    }

    #[test]
    fn should_construct_document_with_all_fields() {
        let doc = Document {
            id: 1,
            source: DocumentSource::Scrape,
            source_ref: "https://example.com".into(),
            content: "hello".into(),
            metadata: Some(r#"{"tags":["news"]}"#.into()),
            embedding: Some(vec![0.1; EMBEDDING_DIM]),
        };
        assert_eq!(doc.id, 1);
        assert_eq!(doc.source, DocumentSource::Scrape);
        assert!(
            doc.embedding
                .as_ref()
                .is_some_and(|e| e.len() == EMBEDDING_DIM)
        );
    }

    #[test]
    fn should_construct_scored_document() {
        let doc = Document {
            id: 2,
            source: DocumentSource::Manual,
            source_ref: "upload.pdf".into(),
            content: "some content".into(),
            metadata: None,
            embedding: None,
        };
        let scored = ScoredDocument {
            document: doc.clone(),
            similarity: 0.95,
        };
        assert_eq!(scored.document.id, doc.id);
        assert!((scored.similarity - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn should_construct_persona_with_all_fields() {
        let persona = Persona {
            id: 1,
            version: 3,
            name: "gaspare".into(),
            system_prompt: "You are Gaspare Spontini.".into(),
            tone: Some("warm".into()),
            fallback_message: Some("Non lo so".into()),
            is_active: true,
            created_at: "2026-07-09T00:00:00Z".into(),
            created_by: Some("admin".into()),
        };
        assert_eq!(persona.id, 1);
        assert_eq!(persona.version, 3);
        assert!(persona.is_active);
    }

    #[test]
    fn should_construct_new_persona_with_required_fields() {
        let new_p = NewPersona {
            name: "gaspare".into(),
            system_prompt: "You are helpful.".into(),
            tone: None,
            fallback_message: None,
            created_by: None,
        };
        assert_eq!(new_p.name, "gaspare");
    }
}
