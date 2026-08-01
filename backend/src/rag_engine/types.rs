use std::fmt;

/// A chunk retrieved from the knowledge base, ordered by similarity.
///
/// This is the rag-engine's own domain type. The retrieval adapter converts
/// `kb_store::ScoredDocument` into this type so the rag-engine never imports
/// `kb_store` (clean-architecture boundary).
#[derive(Debug, Clone, PartialEq)]
pub struct RetrievedChunk {
    pub id: i64,
    pub source_ref: String,
    pub content: String,
    pub similarity: f64,
}

/// A cited source in the generation response.
///
/// The frontend renders this as an expandable inline reference. The `document_id`
/// ties back to the `documents` table; the `source_ref` is the human-readable
/// title (URL, filename, or upload label).
#[derive(Debug, Clone, PartialEq)]
pub struct CitedSource {
    pub document_id: i64,
    pub source_ref: String,
}

/// The answer returned by `RagEngine::answer()`.
///
/// `fell_back` is `true` when the honest-unknown path was taken (no chunks
/// retrieved above the threshold, or the persona fallback was used). The
/// frontend uses this flag to render the "I don't know" state.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub text: String,
    pub sources: Vec<CitedSource>,
    pub fell_back: bool,
}

/// The three structurally separated parts of the prompt sent to `llama-generate`.
///
/// The 3-part separation is non-negotiable per the `spontini-rag-build` skill:
///
/// - `system` — persona instructions ONLY, optionally extended with
///   `/train`-curated training notes (ADR 0016) since those are also
///   instructions, not retrieved content; never contains chunks, never the
///   question.
/// - `context` — retrieved chunks ONLY; never contains persona, never the question.
/// - `user` — the citizen question ONLY; never contains persona, never chunks.
///
/// The `GenerationAdapter` is the ONLY code path that concatenates these into
/// chat messages. No other code may merge the parts.
#[derive(Debug, Clone, PartialEq)]
pub struct PromptParts {
    pub system: String,
    pub context: String,
    pub user: String,
}

/// A read-only projection of the active persona.
///
/// The rag-engine does not depend on the full `kb_store::Persona` type. The
/// persona adapter converts it into this snapshot. If no active persona exists,
/// `RagError::NoActivePersona` is returned before a snapshot is created.
#[derive(Debug, Clone, PartialEq)]
pub struct PersonaSnapshot {
    pub name: String,
    pub system_prompt: String,
    pub fallback_message: Option<String>,
}

/// An admin-readable persona snapshot (all fields).
///
/// The admin API returns this type so it never depends on `kb_store::Persona`.
/// The adapter converts between this domain type and the storage type.
#[derive(Debug, Clone, PartialEq)]
pub struct AdminPersonaSnapshot {
    pub id: i64,
    pub name: String,
    pub version: i32,
    pub system_prompt: String,
    pub tone: Option<String>,
    pub fallback_message: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub created_by: Option<String>,
}

/// Request to create a new persona version.
///
/// The admin API receives this type. The adapter converts it into
/// `kb_store::NewPersona` for persistence, adding `created_by`.
pub struct NewPersonaRequest {
    pub name: String,
    pub system_prompt: String,
    pub tone: Option<String>,
    pub fallback_message: Option<String>,
    pub activate: bool,
    /// TODO(0027): use authenticated admin identity
    pub created_by: Option<String>,
}

/// Errors that can occur in the rag-engine flow.
#[derive(Debug, thiserror::Error)]
pub enum RagError {
    #[error("embedding service error: {0}")]
    Embedding(String),

    #[error("generation service error: {0}")]
    Generation(String),

    #[error("retrieval error: {0}")]
    Retrieval(String),

    #[error("persona error: {0}")]
    Persona(String),

    #[error("persona not found")]
    PersonaNotFound,

    #[error("cannot delete the active persona version")]
    PersonaActive,

    #[error("no active persona configured")]
    NoActivePersona,
}

impl fmt::Display for RetrievedChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RetrievedChunk(id={}, source_ref={:?}, similarity={:.4})",
            self.id, self.source_ref, self.similarity
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_display_retrieved_chunk_with_id_and_source_ref() {
        let chunk = RetrievedChunk {
            id: 42,
            source_ref: "orari-anagrafe.md".into(),
            content: "Contenuto.".into(),
            similarity: 0.87,
        };
        let display = chunk.to_string();
        assert!(display.contains("42"), "display should include id");
        assert!(
            display.contains("orari-anagrafe.md"),
            "display should include source_ref"
        );
    }

    #[test]
    fn should_format_rag_error_display_for_all_variants() {
        assert_eq!(
            RagError::Embedding("timeout".into()).to_string(),
            "embedding service error: timeout"
        );
        assert_eq!(
            RagError::Generation("500".into()).to_string(),
            "generation service error: 500"
        );
        assert_eq!(
            RagError::Retrieval("db".into()).to_string(),
            "retrieval error: db"
        );
        assert_eq!(
            RagError::Persona("missing".into()).to_string(),
            "persona error: missing"
        );
        assert_eq!(RagError::PersonaNotFound.to_string(), "persona not found");
        assert_eq!(
            RagError::NoActivePersona.to_string(),
            "no active persona configured"
        );
    }

    #[test]
    fn should_construct_answer_with_all_fields() {
        let answer = Answer {
            text: "Lo sportello è aperto.".into(),
            sources: vec![CitedSource {
                document_id: 1,
                source_ref: "orari.md".into(),
            }],
            fell_back: false,
        };
        assert!(!answer.fell_back);
        assert_eq!(answer.sources.len(), 1);
        assert_eq!(answer.sources[0].document_id, 1);
    }

    #[test]
    fn should_construct_prompt_parts_with_separated_fields() {
        let parts = PromptParts {
            system: "Sei Gaspare Spontini.".into(),
            context: "Orari: 9-12:30".into(),
            user: "A che ore apre?".into(),
        };
        assert_eq!(parts.system, "Sei Gaspare Spontini.");
        assert_eq!(parts.context, "Orari: 9-12:30");
        assert_eq!(parts.user, "A che ore apre?");
    }

    #[test]
    fn should_construct_persona_snapshot() {
        let snapshot = PersonaSnapshot {
            name: "gaspare".into(),
            system_prompt: "Sei utile.".into(),
            fallback_message: Some("Non lo so.".into()),
        };
        assert_eq!(snapshot.name, "gaspare");
        assert_eq!(snapshot.system_prompt, "Sei utile.");
        assert_eq!(snapshot.fallback_message.as_deref(), Some("Non lo so."));
    }

    #[test]
    fn should_construct_retrieved_chunk_with_similarity() {
        let chunk = RetrievedChunk {
            id: 1,
            source_ref: "test".into(),
            content: "content".into(),
            similarity: 0.95,
        };
        assert_eq!(chunk.id, 1);
        assert!((chunk.similarity - 0.95).abs() < f64::EPSILON);
    }
}
