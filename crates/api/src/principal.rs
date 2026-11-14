//! Authenticated agent principal extractor and security enforcement.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{Method, StatusCode, header};
use chrono::Utc;
use uuid::Uuid;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::auth_routes::SESSION_COOKIE;
use crate::envelope::FeErrorResponse;
use crate::middleware::{CORRELATION_ID_HEADER, REQUEST_ID_HEADER};
use crate::state::AppState;
use iaop_agent_bff_infrastructure::oidc;
use iaop_agent_bff_infrastructure::session::Session;

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

/// Refresh access tokens this many seconds before expiry.
const REFRESH_SKEW: chrono::Duration = chrono::Duration::seconds(60);

/// Per-session async locks so concurrent requests don't race refresh-token
/// rotation. Process-local; multi-replica deployments need a shared lock
/// (documented limitation — refresh rotation is still atomic at the store).
fn session_refresh_lock(session_id: &str) -> Arc<tokio::sync::Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> = OnceLock::new();
    let map = LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut g = map.lock().unwrap_or_else(|e| e.into_inner());
    g.entry(session_id.to_string())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

/// Ensures the session carries an unexpired access token, refreshing via the
/// OIDC token endpoint when needed. Fails closed: revoked/expired refresh
/// tokens invalidate the session; transient IdP outages return 503 without
/// destroying the session.
async fn ensure_fresh_session(
    auth: &crate::auth_routes::AuthContext,
    mut session: Session,
    req_id: Option<uuid::Uuid>,
    corr_id: Option<uuid::Uuid>,
) -> Result<Session, FeErrorResponse> {
    if session.access_expires_at > Utc::now() + REFRESH_SKEW {
        return Ok(session);
    }
    if session.refresh_token.is_none() {
        return Err(FeErrorResponse::unauthenticated(
            "Session expired. Please sign in again.",
            req_id,
            corr_id,
        ));
    }

    // Singleflight per session: concurrent requests wait, then re-check.
    let lock = session_refresh_lock(&session.id);
    let _guard = lock.lock().await;

    // Re-read: another request may have already refreshed.
    if let Some(fresh) = auth.store.get_session(&session.id).await {
        if fresh.access_expires_at > Utc::now() + REFRESH_SKEW {
            return Ok(fresh);
        }
        session = fresh;
    } else {
        return Err(FeErrorResponse::unauthenticated(
            "Session expired or invalid. Please sign in again.",
            req_id,
            corr_id,
        ));
    }
    let Some(rt) = session.refresh_token.clone() else {
        return Err(FeErrorResponse::unauthenticated(
            "Session expired. Please sign in again.",
            req_id,
            corr_id,
        ));
    };

    match oidc::refresh_tokens(&auth.oidc, &rt).await {
        Ok(tokens) => {
            let expires_at =
                Utc::now() + chrono::Duration::seconds(tokens.expires_in.unwrap_or(300) as i64);
            let new_refresh = tokens.refresh_token.clone();
            auth.store
                .update_tokens(
                    &session.id,
                    &tokens.access_token,
                    new_refresh.as_deref(),
                    expires_at,
                )
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "failed to persist refreshed tokens");
                    FeErrorResponse::internal(
                        "Failed to persist session token refresh",
                        req_id,
                        corr_id,
                    )
                })?;
            session.access_token = tokens.access_token;
            if let Some(nr) = new_refresh {
                session.refresh_token = Some(nr);
            }
            session.access_expires_at = expires_at;
            Ok(session)
        }
        Err(oidc::OidcError::TokenEndpoint) => {
            // Refresh token rejected (revoked/expired) — invalidate session.
            auth.store.delete_session(&session.id).await;
            Err(FeErrorResponse::unauthenticated(
                "Session expired. Please sign in again.",
                req_id,
                corr_id,
            ))
        }
        Err(e) => {
            tracing::warn!(error = %e, "token refresh transport failure");
            Err(FeErrorResponse::dependency_unavailable(
                "Identity provider temporarily unavailable",
                req_id,
                corr_id,
            ))
        }
    }
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

        let session = match auth.store.get_session(&sid).await {
            Some(s) => s,
            None => {
                return Err(FeErrorResponse::unauthenticated(
                    "Session expired or invalid. Please sign in again.",
                    req_id,
                    corr_id,
                ));
            }
        };
        let mut session = ensure_fresh_session(auth, session, req_id, corr_id).await?;

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
        let _ = auth.store.update_session(&session).await;

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
