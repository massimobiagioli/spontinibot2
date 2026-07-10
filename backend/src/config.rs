#[derive(Clone)]
pub struct Config {
    pub embed_url: String,
    pub generate_url: String,
    pub kb_path: String,
    pub top_k: i64,
    pub min_score: f64,
    pub admin_api_key: String,
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
            admin_api_key: std::env::var("ADMIN_API_KEY").unwrap_or_else(|_| "dev-key".into()),
        }
    }
}
