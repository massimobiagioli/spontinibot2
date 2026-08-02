use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;

use crate::admin::ErrorResponse;
use crate::admin::scraper_options::{
    RobotsBypassHostResponse, ScraperOptionsAdminPort, ScraperOptionsError,
};
use crate::audit::AuditLogPort;
use crate::audit::record_best_effort;
use crate::auth::extractor::OperatorSession;

#[derive(Clone)]
pub struct ScraperOptionsState {
    pub scraper_options: Arc<dyn ScraperOptionsAdminPort>,
    pub audit: Arc<dyn AuditLogPort>,
}

#[derive(Deserialize)]
pub struct ReplaceRobotsBypassHostsRequest {
    /// One host per line, exactly as typed into the "Opzioni" > "Scraper"
    /// textarea — normalized server-side (trimmed, lowercased, blank lines
    /// and duplicates dropped) rather than trusting the client to have
    /// already cleaned it up.
    pub hosts_text: String,
}

fn normalize_hosts(hosts_text: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut hosts = Vec::new();
    for line in hosts_text.lines() {
        let host = line.trim().to_lowercase();
        if host.is_empty() {
            continue;
        }
        if seen.insert(host.clone()) {
            hosts.push(host);
        }
    }
    hosts
}

fn map_error(e: ScraperOptionsError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        ScraperOptionsError::DbError(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: msg }),
        ),
    }
}

pub async fn list_robots_bypass_hosts(
    State(state): State<ScraperOptionsState>,
    _session: OperatorSession,
) -> Result<Json<Vec<RobotsBypassHostResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let hosts = state
        .scraper_options
        .list_robots_bypass_hosts()
        .await
        .map_err(map_error)?;
    Ok(Json(hosts))
}

pub async fn replace_robots_bypass_hosts(
    State(state): State<ScraperOptionsState>,
    session: OperatorSession,
    Json(req): Json<ReplaceRobotsBypassHostsRequest>,
) -> Result<Json<Vec<RobotsBypassHostResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let hosts = normalize_hosts(&req.hosts_text);
    let response = state
        .scraper_options
        .replace_robots_bypass_hosts(hosts)
        .await
        .map_err(map_error)?;
    record_best_effort(
        state.audit.as_ref(),
        &session.actor,
        "replace_robots_bypass_hosts",
        "robots_bypass_host",
        &serde_json::to_value(&response).unwrap_or_default(),
    )
    .await;
    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_trim_lowercase_dedupe_and_drop_blank_lines() {
        let hosts = normalize_hosts(
            "  Example.com  \n\nexample.com\nOTHER.example.org\n   \nthird.example.net",
        );
        assert_eq!(
            hosts,
            vec![
                "example.com".to_string(),
                "other.example.org".to_string(),
                "third.example.net".to_string(),
            ]
        );
    }

    #[test]
    fn should_return_empty_list_for_blank_input() {
        assert!(normalize_hosts("   \n\n  \n").is_empty());
        assert!(normalize_hosts("").is_empty());
    }
}
