#[derive(Clone)]
pub struct Config {
    pub embed_url: String,
    pub generate_url: String,
    pub kb_path: String,
    pub top_k: i64,
    pub min_score: f64,
    pub operator_credential_path: String,
    pub operator_username: Option<String>,
    pub operator_password: Option<String>,
    pub session_ttl_secs: i64,
    pub upload_max_bytes: usize,
    pub curation_allowed_hosts: Vec<String>,
    pub training_notes_dir: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            embed_url: std::env::var("LLAMA_EMBED_URL")
                .unwrap_or_else(|_| "http://llama-embed:8080".into()),
            generate_url: std::env::var("LLAMA_GENERATE_URL")
                .unwrap_or_else(|_| "http://llama-generate:8080".into()),
            kb_path: std::env::var("KB_DB_PATH").unwrap_or_else(|_| "/data/kb.db".into()),
            top_k: std::env::var("RAG_TOP_K")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            min_score: std::env::var("RAG_MIN_SCORE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.35),
            operator_credential_path: std::env::var("OPERATOR_CREDENTIAL_PATH")
                .unwrap_or_else(|_| "/data/operator-credential.json".into()),
            operator_username: std::env::var("OPERATOR_USERNAME").ok(),
            operator_password: std::env::var("OPERATOR_PASSWORD").ok(),
            session_ttl_secs: std::env::var("SESSION_TTL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1800), // 30 minutes default
            upload_max_bytes: std::env::var("UPLOAD_MAX_BYTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10_485_760), // 10 MB default
            curation_allowed_hosts: parse_curation_hosts(
                std::env::var("CURATION_ALLOWED_HOSTS").ok(),
            ),
            // `/train` writes curated instructional notes into this directory
            // (bind-mounted from the repo's `.project/training/` in
            // docker-compose.yml — see ADR 0016). Read fresh from disk on
            // every chat request (no cache, no reload endpoint needed): a
            // missing or empty directory is not an error, just no notes yet.
            training_notes_dir: std::env::var("TRAINING_NOTES_DIR")
                .unwrap_or_else(|_| "/app/training".into()),
        }
    }
}

/// Parses `CURATION_ALLOWED_HOSTS` (comma-separated hostnames) into the
/// small, explicit allow-list of domains eligible for the non-interactive
/// curation path (Plan 0030) instead of the robots.txt-honoring scrape path.
/// No domain is hardcoded here or anywhere in the curation code — the
/// allow-list is empty (curation effectively off) unless a deployment's own
/// `docker-compose.yml`/env explicitly sets it. A pure function so it's
/// testable without mutating global process env.
fn parse_curation_hosts(raw: Option<String>) -> Vec<String> {
    raw.map(|v| {
        v.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_be_empty_when_no_env_var_set() {
        assert_eq!(parse_curation_hosts(None), Vec::<String>::new());
    }

    #[test]
    fn should_be_empty_when_env_var_is_blank() {
        assert_eq!(
            parse_curation_hosts(Some("".to_string())),
            Vec::<String>::new()
        );
    }

    #[test]
    fn should_parse_comma_separated_hosts_from_env_var() {
        let result = parse_curation_hosts(Some("example.gov.it, other.example.org".to_string()));
        assert_eq!(result, vec!["example.gov.it", "other.example.org"]);
    }
}
