use crate::rag_engine::types::{PersonaSnapshot, PromptParts, RetrievedChunk};

pub fn assemble(
    persona: &PersonaSnapshot,
    chunks: &[RetrievedChunk],
    question: &str,
    training_notes: &str,
) -> PromptParts {
    let context = chunks
        .iter()
        .map(|c| format!("[Fonte: {}]\n{}", c.source_ref, c.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let system = if training_notes.trim().is_empty() {
        persona.system_prompt.clone()
    } else {
        format!(
            "{}\n\n--- Note di addestramento ---\n\n{}",
            persona.system_prompt,
            training_notes.trim()
        )
    };

    PromptParts {
        system,
        context,
        user: question.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_chunks() -> Vec<RetrievedChunk> {
        vec![
            RetrievedChunk {
                id: 1,
                content: "L'anagrafe apre alle 9:00.".into(),
                source_ref: "orari.md".into(),
                similarity: 0.85,
            },
            RetrievedChunk {
                id: 2,
                content: "Chiude alle 12:30.".into(),
                source_ref: "contatti.md".into(),
                similarity: 0.72,
            },
        ]
    }

    fn sample_persona() -> PersonaSnapshot {
        PersonaSnapshot {
            name: "gaspare".into(),
            system_prompt: "Sei Gaspare Spontini.".into(),
            fallback_message: None,
        }
    }

    #[test]
    fn should_place_persona_in_system_only() {
        let persona = sample_persona();
        let chunks = sample_chunks();
        let prompt = assemble(&persona, &chunks, "A che ora apre l'anagrafe?", "");

        assert_eq!(prompt.system, "Sei Gaspare Spontini.");
        assert!(!prompt.context.contains("Sei Gaspare Spontini."));
        assert!(!prompt.user.contains("Sei Gaspare Spontini."));
    }

    #[test]
    fn should_place_question_in_user_only() {
        let persona = sample_persona();
        let chunks = sample_chunks();
        let prompt = assemble(&persona, &chunks, "A che ora apre l'anagrafe?", "");

        assert_eq!(prompt.user, "A che ora apre l'anagrafe?");
        assert!(!prompt.system.contains("A che ora apre l'anagrafe?"));
        assert!(!prompt.context.contains("A che ora apre l'anagrafe?"));
    }

    #[test]
    fn should_place_chunks_in_context_only_with_source_prefix() {
        let persona = sample_persona();
        let chunks = sample_chunks();
        let prompt = assemble(&persona, &chunks, "A che ora apre l'anagrafe?", "");

        assert!(prompt.context.contains("[Fonte: orari.md]"));
        assert!(prompt.context.contains("L'anagrafe apre alle 9:00."));
        assert!(prompt.context.contains("[Fonte: contatti.md]"));
        assert!(prompt.context.contains("Chiude alle 12:30."));
        assert!(!prompt.system.contains("L'anagrafe apre alle 9:00."));
        assert!(!prompt.user.contains("L'anagrafe apre alle 9:00."));
    }

    #[test]
    fn should_join_multiple_chunks_with_separator() {
        let persona = sample_persona();
        let chunks = sample_chunks();
        let prompt = assemble(&persona, &chunks, "test", "");

        assert!(prompt.context.contains("\n\n---\n\n"));
        let parts: Vec<&str> = prompt.context.split("\n\n---\n\n").collect();
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn should_produce_empty_context_for_no_chunks() {
        let persona = sample_persona();
        let prompt = assemble(&persona, &[], "test", "");

        assert_eq!(prompt.context, "");
        assert_eq!(prompt.system, "Sei Gaspare Spontini.");
        assert_eq!(prompt.user, "test");
    }

    #[test]
    fn should_append_training_notes_to_system_when_present() {
        let persona = sample_persona();
        let chunks = sample_chunks();
        let prompt = assemble(
            &persona,
            &chunks,
            "test",
            "Non inventare mai un orario non presente nel contesto.",
        );

        assert!(prompt.system.starts_with("Sei Gaspare Spontini."));
        assert!(
            prompt
                .system
                .contains("Non inventare mai un orario non presente nel contesto.")
        );
        assert!(!prompt.context.contains("Non inventare mai"));
        assert!(!prompt.user.contains("Non inventare mai"));
    }

    #[test]
    fn should_leave_system_unchanged_when_training_notes_is_blank() {
        let persona = sample_persona();
        let chunks = sample_chunks();
        let prompt = assemble(&persona, &chunks, "test", "   \n  ");

        assert_eq!(prompt.system, "Sei Gaspare Spontini.");
    }
}
