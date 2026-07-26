/// Canonical, literal phrasings recognized as an identity/imprinting question
/// (ADR 0014) — checked against the normalized question text (lowercased,
/// trimmed, trailing punctuation stripped). This is deliberately a small,
/// closed set, not a classifier: a question that doesn't match here simply
/// falls through to the normal RAG flow and is still answered correctly,
/// just without the instant-answer latency benefit.
const IDENTITY_PHRASES: &[&str] = &[
    "chi sei",
    "chi sei tu",
    "chi sei esattamente",
    "cosa sei",
    "come ti chiami",
    "qual è il tuo nome",
    "qual e il tuo nome",
    "presentati",
    "parlami di te",
    "dimmi chi sei",
];

/// Normalizes a question for identity-phrase matching: lowercase, trim,
/// strip trailing `?`/`!`/`.`/`…`, collapse internal whitespace runs.
fn normalize(text: &str) -> String {
    let lowered = text.trim().to_lowercase();
    let trimmed = lowered.trim_end_matches(['?', '!', '.', '…']).trim();
    trimmed.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// True if `question` is a literal identity/imprinting question about the
/// bot itself — either one of the fixed canonical phrasings, or "chi è
/// `<persona_name>`" for the currently active persona's own name.
pub fn is_identity_question(question: &str, persona_name: &str) -> bool {
    let normalized = normalize(question);
    if IDENTITY_PHRASES.contains(&normalized.as_str()) {
        return true;
    }

    let name_normalized = normalize(persona_name);
    if name_normalized.is_empty() {
        return false;
    }
    normalized == format!("chi è {name_normalized}")
        || normalized == format!("chi è {name_normalized} esattamente")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_match_canonical_identity_phrasings_case_and_punctuation_insensitively() {
        assert!(is_identity_question("Chi sei?", "Gaspare"));
        assert!(is_identity_question("chi sei", "Gaspare"));
        assert!(is_identity_question("CHI SEI TU?!", "Gaspare"));
        assert!(is_identity_question("  Come ti chiami?  ", "Gaspare"));
        assert!(is_identity_question("Qual è il tuo nome?", "Gaspare"));
        assert!(is_identity_question("Presentati.", "Gaspare"));
        assert!(is_identity_question("Parlami di te", "Gaspare"));
    }

    #[test]
    fn should_match_who_is_persona_name_parametrized_by_the_active_persona() {
        assert!(is_identity_question("Chi è Gaspare?", "Gaspare"));
        assert!(is_identity_question("chi è gaspare", "Gaspare"));
        assert!(is_identity_question(
            "Chi è Gaspare esattamente?",
            "Gaspare"
        ));
        // a different persona name must not match the wrong one
        assert!(!is_identity_question("Chi è Gaspare?", "SpontiniBot"));
    }

    #[test]
    fn should_not_match_unrelated_or_municipal_questions() {
        assert!(!is_identity_question(
            "A che ora apre l'anagrafe?",
            "Gaspare"
        ));
        assert!(!is_identity_question("Chi è il sindaco?", "Gaspare"));
        assert!(!is_identity_question(
            "Quando è nato Gaspare Spontini?",
            "Gaspare"
        ));
        assert!(!is_identity_question("", "Gaspare"));
    }

    #[test]
    fn should_not_match_when_persona_name_is_empty() {
        assert!(!is_identity_question("Chi è ?", ""));
    }
}
