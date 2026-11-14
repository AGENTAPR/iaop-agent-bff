//! Backend-managed OIDC auth endpoints (Authorization Code + PKCE).
//! Browser never sees tokens; session is an opaque HttpOnly cookie.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use serde::Deserialize;

use iaop_agent_bff_infrastructure::oidc::{
    self, OidcConfig, OidcError, exchange_code, pkce_challenge, random_token,
};
use iaop_agent_bff_infrastructure::session::{
    LoginTransaction, Session, SessionStore, new_session_id,
};
use iaop_security::jwks::{JwtValidator, RemoteJwksProvider};

use crate::state::AppState;

pub const SESSION_COOKIE: &str = "__Host-iaop_session";
pub const BINDING_COOKIE: &str = "__Host-iaop_login_binding";
const TXN_TTL: Duration = Duration::from_secs(300);

/// Optional OIDC runtime context; `None` => auth routes return 503.
pub struct AuthContext {
    pub oidc: OidcConfig,
    pub store: Arc<dyn SessionStore>,
    /// Validates ID tokens (audience = BFF client_id).
    pub id_validator: JwtValidator<RemoteJwksProvider>,
}

impl AppState {
    #[allow(clippy::result_large_err)]
    pub fn auth(&self) -> Result<&AuthContext, Response> {
        self.auth.as_deref().ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error":"auth not configured"})),
            )
                .into_response()
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    pub return_to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

fn cookie_secure(state: &AppState) -> bool {
    state.config.app_base_url.starts_with("https://")
}

fn set_cookie(name: &str, value: &str, max_age_secs: i64, secure: bool) -> String {
    format!(
        "{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age_secs}{}",
        if secure { "; Secure" } else { "" }
    )
}

fn clear_cookie(name: &str, secure: bool) -> String {
    format!(
        "{name}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
        if secure { "; Secure" } else { "" }
    )
}

fn read_cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|p| p.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v)
}

fn err(status: StatusCode, msg: &str) -> Response {
    (status, Json(serde_json::json!({"error": msg}))).into_response()
}

/// GET /auth/login?return_to=/path — initiates Authorization Code + PKCE.
pub async fn login(
    State(state): State<AppState>,
    Query(q): Query<LoginQuery>,
    headers: HeaderMap,
) -> Response {
    let auth = match state.auth() {
        Ok(a) => a,
        Err(r) => return r,
    };
    let return_to = auth.oidc.sanitize_return_to(q.return_to.as_deref());

    let state_param = random_token();
    let nonce = random_token();
    let verifier = random_token();
    let challenge = pkce_challenge(&verifier);
    // Browser binding ties the transaction to the initiating browser.
    let binding = random_token();

    let tx = LoginTransaction {
        state: state_param.clone(),
        nonce: nonce.clone(),
        pkce_verifier: verifier,
        return_to,
        browser_binding: binding.clone(),
        created_at: std::time::Instant::now(),
        ttl: TXN_TTL,
    };
    if auth.store.put_login_txn(tx).await.is_err() {
        return err(StatusCode::INTERNAL_SERVER_ERROR, "session store error");
    }

    let url = oidc::authorize_url(&auth.oidc, &state_param, &nonce, &challenge);
    let mut resp = Redirect::to(&url).into_response();
    resp.headers_mut().append(
        header::SET_COOKIE,
        set_cookie(BINDING_COOKIE, &binding, 300, cookie_secure(&state))
            .parse()
            .unwrap(),
    );
    let _ = headers;
    resp
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct IdTokenClaims {
    sub: String,
    iss: String,
    aud: serde_json::Value,
    exp: usize,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    preferred_username: Option<String>,
}

/// GET /auth/callback?code&state — completes login, creates session.
pub async fn callback(
    State(state): State<AppState>,
    Query(q): Query<CallbackQuery>,
    headers: HeaderMap,
) -> Response {
    let auth = match state.auth() {
        Ok(a) => a,
        Err(r) => return r,
    };
    if let Some(e) = q.error {
        // Never log the code; error param is safe to classify only.
        tracing::warn!(error = %e, "oidc callback error");
        return err(StatusCode::BAD_REQUEST, "authentication failed");
    }
    let (Some(code), Some(state_param)) = (q.code, q.state) else {
        return err(StatusCode::BAD_REQUEST, "missing code or state");
    };

    // Single-use state + browser binding check.
    let Some(tx) = auth.store.take_login_txn(&state_param).await else {
        return err(StatusCode::BAD_REQUEST, "invalid or expired state");
    };
    let bound = read_cookie(&headers, BINDING_COOKIE);
    if bound != Some(tx.browser_binding.as_str()) {
        return err(StatusCode::FORBIDDEN, "login transaction binding mismatch");
    }

    let tokens = match exchange_code(&auth.oidc, &code, &tx.pkce_verifier).await {
        Ok(t) => t,
        Err(OidcError::TokenEndpoint) => {
            return err(
                StatusCode::BAD_GATEWAY,
                "identity provider rejected token exchange",
            );
        }
        Err(_) => return err(StatusCode::BAD_GATEWAY, "identity provider unavailable"),
    };

    // Validate ID token signature/iss/aud/exp via shared JWKS validator.
    let claims = match auth
        .id_validator
        .validate::<IdTokenClaims>(&tokens.id_token)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "id token validation failed");
            return err(StatusCode::UNAUTHORIZED, "invalid identity token");
        }
    };
    if claims.nonce.as_deref() != Some(tx.nonce.as_str()) {
        return err(StatusCode::UNAUTHORIZED, "nonce mismatch");
    }

    let session_id = new_session_id();
    let session = Session {
        id: session_id.clone(),
        subject: claims.sub.clone(),
        display_name: claims.name.clone().or(claims.preferred_username.clone()),
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        access_expires_at: Utc::now()
            + chrono::Duration::seconds(tokens.expires_in.unwrap_or(300) as i64),
        created_at: std::time::Instant::now(),
        last_seen: std::time::Instant::now(),
        idle_timeout: Duration::from_secs(state.config.session_idle_timeout_secs),
        absolute_timeout: Duration::from_secs(state.config.session_absolute_timeout_secs),
        csrf_token: random_token(),
        workspace_id: None,
        agent_id: None,
    };
    if auth.store.create_session(session).await.is_err() {
        return err(StatusCode::INTERNAL_SERVER_ERROR, "session store error");
    }
    tracing::info!(subject = %claims.sub, "login succeeded");

    let mut resp = Redirect::to(&tx.return_to).into_response();
    let h = resp.headers_mut();
    h.append(
        header::SET_COOKIE,
        set_cookie(SESSION_COOKIE, &session_id, 43200, cookie_secure(&state))
            .parse()
            .unwrap(),
    );
    h.append(
        header::SET_COOKIE,
        clear_cookie(BINDING_COOKIE, cookie_secure(&state))
            .parse()
            .unwrap(),
    );
    resp
}

#[derive(Debug, Serialize)]
struct SessionInfo {
    authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<UserInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    csrf_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct UserInfo {
    subject: String,
    display_name: Option<String>,
}

use serde::Serialize;

#[allow(clippy::result_large_err)]
async fn session_from_request<'a>(
    state: &'a AppState,
    headers: &'a HeaderMap,
) -> Result<
    (
        &'a AuthContext,
        iaop_agent_bff_infrastructure::session::Session,
    ),
    Response,
> {
    let auth = state.auth()?;
    let sid = read_cookie(headers, SESSION_COOKIE)
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "no session"))?;
    let mut s = auth
        .store
        .get_session(sid)
        .await
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "session expired"))?;
    s.last_seen = std::time::Instant::now();
    let _ = auth.store.update_session(&s).await;
    Ok((auth, s))
}

/// GET /auth/session — safe session info only.
pub async fn session_info(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match session_from_request(&state, &headers).await {
        Ok((_, s)) => Json(SessionInfo {
            authenticated: true,
            user: Some(UserInfo {
                subject: s.subject,
                display_name: s.display_name,
            }),
            csrf_token: Some(s.csrf_token),
            expires_at: Some(s.access_expires_at),
        })
        .into_response(),
        Err(_) => Json(SessionInfo {
            authenticated: false,
            user: None,
            csrf_token: None,
            expires_at: None,
        })
        .into_response(),
    }
}

/// GET /auth/me — alias returning the same safe principal info.
pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> Response {
    session_info(State(state), headers).await
}

/// POST /auth/logout — requires CSRF header; destroys local session.
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let auth = match state.auth() {
        Ok(a) => a,
        Err(r) => return r,
    };
    let Some(sid) = read_cookie(&headers, SESSION_COOKIE) else {
        return err(StatusCode::UNAUTHORIZED, "no session");
    };
    // CSRF: state-changing request must carry the session's csrf token.
    let csrf = headers
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let session = auth.store.get_session(sid).await;
    match session {
        Some(s) if s.csrf_token == csrf => {
            auth.store.delete_session(sid).await;
            tracing::info!("logout: session revoked");
            let mut resp = Json(serde_json::json!({"ok": true})).into_response();
            resp.headers_mut().append(
                header::SET_COOKIE,
                clear_cookie(SESSION_COOKIE, cookie_secure(&state))
                    .parse()
                    .unwrap(),
            );
            resp
        }
        Some(_) => err(StatusCode::FORBIDDEN, "csrf validation failed"),
        None => err(StatusCode::UNAUTHORIZED, "session expired"),
    }
}
