use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, FromRequestParts};
use axum::http::StatusCode;
use axum::http::request::Parts;

use crate::admin::ErrorResponse;
use crate::auth::session_store::SessionStore;

pub struct OperatorSession {
    pub actor: String,
}

/// Parses the `session=<token>` cookie out of a raw `Cookie` header value.
pub fn extract_session_token(cookie_header: &str) -> Option<String> {
    cookie_header.split(';').find_map(|part| {
        let part = part.trim();
        part.strip_prefix("session=").map(|token| token.to_string())
    })
}

/// Authorizes a request given its raw `Cookie` header value (if any) and the
/// shared session store. Pure and directly testable — the axum extractor
/// below is a thin wrapper around this.
pub fn authorize(
    store: &SessionStore,
    cookie_header: Option<&str>,
) -> Result<OperatorSession, (StatusCode, Json<ErrorResponse>)> {
    let token = cookie_header
        .and_then(extract_session_token)
        .ok_or_else(unauthorized)?;
    let record = store.get(&token).ok_or_else(unauthorized)?;
    Ok(OperatorSession {
        actor: record.actor,
    })
}

fn unauthorized() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "invalid or missing session cookie".into(),
        }),
    )
}

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for OperatorSession
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Extension(store) = Extension::<Arc<SessionStore>>::from_request_parts(parts, state)
            .await
            .map_err(|_| unauthorized())?;

        let cookie_header = parts
            .headers
            .get(axum::http::header::COOKIE)
            .and_then(|v| v.to_str().ok());

        authorize(&store, cookie_header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_extract_session_token_from_single_cookie() {
        let token = extract_session_token("session=abc123");
        assert_eq!(token.as_deref(), Some("abc123"));
    }

    #[test]
    fn should_extract_session_token_among_multiple_cookies() {
        let token = extract_session_token("foo=bar; session=abc123; baz=qux");
        assert_eq!(token.as_deref(), Some("abc123"));
    }

    #[test]
    fn should_return_none_when_session_cookie_absent() {
        let token = extract_session_token("foo=bar; baz=qux");
        assert!(token.is_none());
    }

    #[test]
    fn should_authorize_with_valid_session_cookie() {
        let store = SessionStore::new(1800);
        let token = store.insert("operator".into());
        let cookie_header = format!("session={token}");

        let session = authorize(&store, Some(&cookie_header)).expect("should authorize");
        assert_eq!(session.actor, "operator");
    }

    #[test]
    fn should_reject_missing_cookie_header() {
        let store = SessionStore::new(1800);
        let result = authorize(&store, None);
        assert!(result.is_err());
    }

    #[test]
    fn should_reject_unknown_session_token() {
        let store = SessionStore::new(1800);
        let result = authorize(&store, Some("session=nonexistent"));
        assert!(result.is_err());
    }
}
