//! Authenticated agent principal extractor and security enforcement.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{Method, StatusCode, header};
use uuid::Uuid;

use crate::auth_routes::SESSION_COOKIE;
use crate::envelope::FeErrorResponse;
use crate::middleware::{CORRELATION_ID_HEADER, REQUEST_ID_HEADER};
use crate::state::AppState;

/// Fallback cookie name for non-HTTPS development environments.
const DEV_SESSION_COOKIE: &str = "iaop_session";
const CSRF_HEADER: &str = "x-csrf-token";

/// Authenticated user principal extracted from the verified session.
#[derive(Debug, Clone)]
pub struct AuthenticatedPrincipal {
    pub session_id: String,
    pub subject: String,
    pub display_name: Option<String>,
    pub csrf_token: String,
    pub workspace_id: Option<String>,
    pub agent_id: Option<Uuid>,
    pub access_token: String,
    pub request_id: Option<Uuid>,
    pub correlation_id: Option<Uuid>,
}

fn read_session_cookie(parts: &Parts) -> Option<String> {
    let cookie_header = parts.headers.get(header::COOKIE)?.to_str().ok()?;
    for pair in cookie_header.split(';') {
        let trimmed = pair.trim();
        if let Some((k, v)) = trimmed.split_once('=') {
            if k == SESSION_COOKIE || k == DEV_SESSION_COOKIE {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn extract_ids(parts: &Parts) -> (Option<Uuid>, Option<Uuid>) {
    let req_id = parts
        .headers
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| Uuid::parse_str(v).ok());
    let corr_id = parts
        .headers
        .get(CORRELATION_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| Uuid::parse_str(v).ok())
        .or(req_id);
    (req_id, corr_id)
}

impl FromRequestParts<AppState> for AuthenticatedPrincipal {
    type Rejection = FeErrorResponse;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let (req_id, corr_id) = extract_ids(parts);

        let auth = match state.auth() {
            Ok(a) => a,
            Err(_) => {
                return Err(FeErrorResponse::dependency_unavailable(
                    "Authentication subsystem unavailable",
                    req_id,
                    corr_id,
                ));
            }
        };

        let sid = match read_session_cookie(parts) {
            Some(id) => id,
            None => {
                return Err(FeErrorResponse::unauthenticated(
                    "Authentication required. Missing session cookie.",
                    req_id,
                    corr_id,
                ));
            }
        };

        let mut session = match auth.store.get_session(&sid) {
            Some(s) => s,
            None => {
                return Err(FeErrorResponse::unauthenticated(
                    "Session expired or invalid. Please sign in again.",
                    req_id,
                    corr_id,
                ));
            }
        };

        // CSRF verification for mutating requests
        let method = &parts.method;
        if method == Method::POST
            || method == Method::PUT
            || method == Method::PATCH
            || method == Method::DELETE
        {
            let csrf_sent = parts
                .headers
                .get(CSRF_HEADER)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default();

            if csrf_sent.is_empty() || csrf_sent != session.csrf_token {
                tracing::warn!(
                    subject = %session.subject,
                    method = %method,
                    path = %parts.uri.path(),
                    "CSRF validation failed for state-changing request"
                );
                return Err(FeErrorResponse::forbidden(
                    "CSRF validation failed. Include valid x-csrf-token header.",
                    req_id,
                    corr_id,
                )
                .with_status(StatusCode::FORBIDDEN));
            }
        }

        // Update session activity timestamp
        session.last_seen = std::time::Instant::now();
        let _ = auth.store.update_session(&session);

        Ok(AuthenticatedPrincipal {
            session_id: session.id,
            subject: session.subject,
            display_name: session.display_name,
            csrf_token: session.csrf_token,
            workspace_id: session.workspace_id,
            agent_id: session.agent_id,
            access_token: session.access_token,
            request_id: req_id,
            correlation_id: corr_id,
        })
    }
}
