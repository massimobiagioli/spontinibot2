//! Automatic tag derivation for manually uploaded documents.
//!
//! The operator no longer picks tags by hand — we derive them from the
//! extracted text itself: the most frequent significant (non-stopword,
//! non-trivial) words, so every upload gets a consistent, unbiased set of
//! tags instead of relying on ad hoc operator input.

const STOPWORDS: &[&str] = &[
    "questo", "questa", "questi", "queste", "quello", "quella", "quelli", "quelle", "perché",
    "come", "dove", "quando", "della", "dello", "delle", "degli", "sono", "erano", "essere",
    "hanno", "abbiamo", "avete", "loro", "nostro", "nostra", "nostri", "nostre", "vostro",
    "vostra", "vostri", "vostre", "anche", "molto", "poco", "tutto", "tutti", "tutta", "tutte",
    "ancora", "sempre", "senza", "sopra", "sotto", "prima", "dopo", "quale", "quali", "ogni",
    "altro", "altra", "altri", "altre", "stesso", "stessa", "stessi", "stesse", "presente",
    "presenti",
];

/// Returns up to `max_tags` significant words from `text`, ranked by
/// frequency (ties broken alphabetically for determinism).
pub fn extract_tags(text: &str, max_tags: usize) -> Vec<String> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for word in text.split(|c: char| !c.is_alphanumeric()) {
        if word.is_empty() {
            continue;
        }
        let normalized = word.to_lowercase();
        if normalized.chars().count() < 5 {
            continue;
        }
        if normalized.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if STOPWORDS.contains(&normalized.as_str()) {
            continue;
        }
        *counts.entry(normalized).or_insert(0) += 1;
    }

    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.into_iter().take(max_tags).map(|(w, _)| w).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_rank_words_by_frequency() {
        let text = "comune comune comune delibera delibera consiglio";
        let tags = extract_tags(text, 5);
        assert_eq!(tags, vec!["comune", "delibera", "consiglio"]);
    }

    #[test]
    fn should_ignore_stopwords() {
        let text = "questo questo questo comune comune comune";
        let tags = extract_tags(text, 5);
        assert_eq!(tags, vec!["comune"]);
    }

    #[test]
    fn should_ignore_short_words_and_pure_numbers() {
        let text = "il 2026 e a di comune comune comune";
        let tags = extract_tags(text, 5);
        assert_eq!(tags, vec!["comune"]);
    }

    #[test]
    fn should_cap_at_max_tags() {
        let text = "alfa alfa beta beta gamma gamma delta delta epsilon epsilon";
        let tags = extract_tags(text, 3);
        assert_eq!(tags.len(), 3);
    }

    #[test]
    fn should_break_ties_alphabetically_for_determinism() {
        let text = "zebra zebra alpha alpha";
        let tags = extract_tags(text, 2);
        assert_eq!(tags, vec!["alpha", "zebra"]);
    }

    #[test]
    fn should_return_empty_for_text_with_no_significant_words() {
        let text = "il la di e a";
        let tags = extract_tags(text, 5);
        assert!(tags.is_empty());
    }
}
