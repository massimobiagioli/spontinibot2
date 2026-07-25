use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use serde::Deserialize;

use crate::admin::ErrorResponse;
use crate::auth::credential::{OperatorCredential, verify_password};
use crate::auth::extractor::extract_session_token;
use crate::auth::session_store::SessionStore;
use crate::config::Config;

#[derive(Clone)]
pub struct AuthState {
    pub config: Config,
    pub session_store: Arc<SessionStore>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

fn session_cookie(token: &str, max_age_secs: i64) -> String {
    format!("session={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={max_age_secs}")
}

const CLEARED_SESSION_COOKIE: &str = "session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0";

pub async fn login(
    State(state): State<AuthState>,
    Json(req): Json<LoginRequest>,
) -> Result<(StatusCode, HeaderMap, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let credential = OperatorCredential::load_from_file(&state.config.operator_credential_path)
        .map_err(|_| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "operator credential not configured".into(),
                }),
            )
        })?;

    if req.username != credential.username
        || !verify_password(&credential.password_hash, &req.password)
    {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "invalid credentials".into(),
            }),
        ));
    }

    let token = state.session_store.insert(credential.username.clone());
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        session_cookie(&token, state.config.session_ttl_secs)
            .parse()
            .expect("cookie header value should be valid"),
    );

    Ok((
        StatusCode::OK,
        headers,
        Json(serde_json::json!({"status": "logged_in"})),
    ))
}

pub async fn logout(
    State(state): State<AuthState>,
    headers: HeaderMap,
) -> (StatusCode, HeaderMap, Json<serde_json::Value>) {
    if let Some(token) = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(extract_session_token)
    {
        state.session_store.remove(&token);
    }

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        CLEARED_SESSION_COOKIE
            .parse()
            .expect("cookie header value should be valid"),
    );

    (
        StatusCode::OK,
        response_headers,
        Json(serde_json::json!({"status": "logged_out"})),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::Argon2;
    use argon2::PasswordHasher;
    use argon2::password_hash::SaltString;
    use rand::rngs::OsRng;

    fn write_credential_file(username: &str, password: &str) -> String {
        let path = std::env::temp_dir().join(format!(
            "spontini_auth_handlers_test_{}_{}.json",
            std::process::id(),
            rand::random::<u32>()
        ));
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .expect("hash failed")
            .to_string();
        std::fs::write(
            &path,
            format!(r#"{{"username":"{username}","password_hash":"{hash}"}}"#),
        )
        .expect("write failed");
        path.to_str().unwrap().to_string()
    }

    fn test_state(credential_path: String) -> AuthState {
        AuthState {
            config: Config {
                embed_url: "http://localhost:8080".into(),
                generate_url: "http://localhost:8081".into(),
                kb_path: "/tmp/test.db".into(),
                top_k: 5,
                min_score: 0.35,
                operator_credential_path: credential_path,
                session_ttl_secs: 1800,
                upload_max_bytes: 10_485_760,
            },
            session_store: Arc::new(SessionStore::new(1800)),
        }
    }

    #[tokio::test]
    async fn should_login_with_correct_credentials() {
        let path = write_credential_file("operator", "s3cret");
        let state = test_state(path.clone());

        let result = login(
            State(state),
            Json(LoginRequest {
                username: "operator".into(),
                password: "s3cret".into(),
            }),
        )
        .await;

        assert!(result.is_ok());
        let (status, headers, _) = result.unwrap();
        assert_eq!(status, StatusCode::OK);
        let set_cookie = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
        assert!(set_cookie.starts_with("session="));
        assert!(set_cookie.contains("HttpOnly"));

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_reject_login_with_wrong_password() {
        let path = write_credential_file("operator", "s3cret");
        let state = test_state(path.clone());

        let result = login(
            State(state),
            Json(LoginRequest {
                username: "operator".into(),
                password: "wrong".into(),
            }),
        )
        .await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_reject_login_with_unknown_username() {
        let path = write_credential_file("operator", "s3cret");
        let state = test_state(path.clone());

        let result = login(
            State(state),
            Json(LoginRequest {
                username: "someone-else".into(),
                password: "s3cret".into(),
            }),
        )
        .await;

        assert!(result.is_err());

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn should_reject_login_when_credential_file_missing() {
        let state = test_state("/nonexistent-spontini-credential.json".into());

        let result = login(
            State(state),
            Json(LoginRequest {
                username: "operator".into(),
                password: "s3cret".into(),
            }),
        )
        .await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn should_logout_and_clear_cookie() {
        let state = test_state("/nonexistent.json".into());
        let token = state.session_store.insert("operator".into());
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, format!("session={token}").parse().unwrap());

        let (status, response_headers, _) = logout(State(state.clone()), headers).await;

        assert_eq!(status, StatusCode::OK);
        let set_cookie = response_headers
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(set_cookie.contains("Max-Age=0"));
        assert!(state.session_store.get(&token).is_none());
    }

    #[tokio::test]
    async fn should_logout_gracefully_without_cookie() {
        let state = test_state("/nonexistent.json".into());

        let (status, _, _) = logout(State(state), HeaderMap::new()).await;

        assert_eq!(status, StatusCode::OK);
    }
}
