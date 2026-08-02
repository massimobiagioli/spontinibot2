pub mod adapter;
pub mod handlers;

use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use kb_store::RobotsBypassHost;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RobotsBypassHostResponse {
    pub id: i64,
    pub host: String,
    pub created_at: String,
}

impl From<RobotsBypassHost> for RobotsBypassHostResponse {
    fn from(h: RobotsBypassHost) -> Self {
        Self {
            id: h.id,
            host: h.host,
            created_at: h.created_at,
        }
    }
}

#[derive(Debug)]
pub enum ScraperOptionsError {
    DbError(String),
}

impl fmt::Display for ScraperOptionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScraperOptionsError::DbError(msg) => write!(f, "database error: {msg}"),
        }
    }
}

impl std::error::Error for ScraperOptionsError {}

impl From<kb_store::KbStoreError> for ScraperOptionsError {
    fn from(e: kb_store::KbStoreError) -> Self {
        ScraperOptionsError::DbError(e.to_string())
    }
}

/// The one operator-editable list of hosts allowed to bypass robots.txt
/// entirely (admin-ui "Opzioni" > "Scraper"). See AGENTS.md's "Scraper
/// Exceptions Must Be Operator-Configured, Never Hard-Coded" rule — this
/// port is the only place that list is allowed to live; nothing in
/// `ingest-core` may hard-code a host here.
#[async_trait]
pub trait ScraperOptionsAdminPort: Send + Sync {
    async fn list_robots_bypass_hosts(
        &self,
    ) -> Result<Vec<RobotsBypassHostResponse>, ScraperOptionsError>;

    /// Replaces the entire list — the admin-ui page is a single multiline
    /// textarea representing the full current set, not a per-item CRUD
    /// flow, so saving always means "this is now the complete list."
    async fn replace_robots_bypass_hosts(
        &self,
        hosts: Vec<String>,
    ) -> Result<Vec<RobotsBypassHostResponse>, ScraperOptionsError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_convert_robots_bypass_host_to_response() {
        let host = RobotsBypassHost {
            id: 1,
            host: "example.com".into(),
            created_at: "2026-08-02 00:00:00".into(),
        };
        let response = RobotsBypassHostResponse::from(host);
        assert_eq!(response.id, 1);
        assert_eq!(response.host, "example.com");
        assert_eq!(response.created_at, "2026-08-02 00:00:00");
    }

    #[test]
    fn should_format_scraper_options_error_display() {
        let err = ScraperOptionsError::DbError("connection refused".into());
        assert_eq!(err.to_string(), "database error: connection refused");
    }
}
