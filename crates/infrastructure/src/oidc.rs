//! OIDC Authorization Code + PKCE client for backend-managed auth.
//! The BFF never exposes tokens to the browser; it exchanges the code
//! server-side and stores tokens only in the server-side session.

use base64::Engine;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OidcError {
    #[error("oidc request failed: {0}")]
    Transport(String),
    #[error("oidc token endpoint rejected request")]
    TokenEndpoint,
    #[error("invalid id token: {0}")]
    InvalidIdToken(String),
    #[error("missing required claim: {0}")]
    MissingClaim(&'static str),
}

/// Keycloak/OIDC client configuration (confidential BFF client).
#[derive(Debug, Clone)]
pub struct OidcConfig {
    /// e.g. https://host/realms/iaop-dev
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    /// Exact registered redirect URI, e.g. https://bff/auth/callback
    pub redirect_uri: String,
    /// Allowed post-login return paths (allowlist prefixes).
    pub allowed_return_paths: Vec<String>,
}

impl OidcConfig {
    pub fn authorize_endpoint(&self) -> String {
        format!("{}/protocol/openid-connect/auth", self.issuer)
    }
    pub fn token_endpoint(&self) -> String {
        format!("{}/protocol/openid-connect/token", self.issuer)
    }
    pub fn end_session_endpoint(&self) -> String {
        format!("{}/protocol/openid-connect/logout", self.issuer)
    }
    pub fn jwks_uri(&self) -> String {
        format!("{}/protocol/openid-connect/certs", self.issuer)
    }

    /// Validates a requested post-login path against the allowlist.
    pub fn sanitize_return_to(&self, path: Option<&str>) -> String {
        match path {
            Some(p) if p.starts_with('/') && !p.starts_with("//") => {
                if self
                    .allowed_return_paths
                    .iter()
                    .any(|a| p == a || p.starts_with(&format!("{a}/")))
                {
                    p.to_string()
                } else {
                    self.allowed_return_paths
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "/".into())
                }
            }
            _ => self
                .allowed_return_paths
                .first()
                .cloned()
                .unwrap_or_else(|| "/".into()),
        }
    }
}

/// Generates a PKCE S256 challenge from a random verifier.
pub fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

/// High-entropy random URL-safe string (32 bytes -> ~43 chars).
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Builds the Keycloak authorization URL with state, nonce, PKCE.
pub fn authorize_url(cfg: &OidcConfig, state: &str, nonce: &str, challenge: &str) -> String {
    let q = [
        ("response_type", "code"),
        ("client_id", cfg.client_id.as_str()),
        ("redirect_uri", cfg.redirect_uri.as_str()),
        ("scope", "openid profile email"),
        ("state", state),
        ("nonce", nonce),
        ("code_challenge", challenge),
        ("code_challenge_method", "S256"),
    ];
    let qs = q
        .iter()
        .map(|(k, v)| format!("{k}={}", urlencoding(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{}?{}", cfg.authorize_endpoint(), qs)
}

fn urlencoding(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Present on code exchange; refresh grants may omit it.
    #[serde(default)]
    pub id_token: String,
}

/// Exchanges an authorization code for tokens (backend-side, confidential client).
pub async fn exchange_code(
    cfg: &OidcConfig,
    code: &str,
    verifier: &str,
) -> Result<TokenResponse, OidcError> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| OidcError::Transport(e.to_string()))?;
    let resp = http
        .post(cfg.token_endpoint())
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", cfg.client_id.as_str()),
            ("client_secret", cfg.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", cfg.redirect_uri.as_str()),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .map_err(|e| OidcError::Transport(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(OidcError::TokenEndpoint);
    }
    resp.json::<TokenResponse>()
        .await
        .map_err(|e| OidcError::Transport(e.to_string()))
}

/// Refreshes an access token using a refresh token.
pub async fn refresh_tokens(
    cfg: &OidcConfig,
    refresh_token: &str,
) -> Result<TokenResponse, OidcError> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| OidcError::Transport(e.to_string()))?;
    let resp = http
        .post(cfg.token_endpoint())
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", cfg.client_id.as_str()),
            ("client_secret", cfg.client_secret.as_str()),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .map_err(|e| OidcError::Transport(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(OidcError::TokenEndpoint);
    }
    resp.json::<TokenResponse>()
        .await
        .map_err(|e| OidcError::Transport(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_is_s256() {
        // RFC 7636 appendix B test vector
        let v = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            pkce_challenge(v),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn return_to_allowlist() {
        let cfg = OidcConfig {
            issuer: "https://idp".into(),
            client_id: "c".into(),
            client_secret: "s".into(),
            redirect_uri: "https://bff/auth/callback".into(),
            allowed_return_paths: vec!["/".into(), "/dashboard".into()],
        };
        assert_eq!(cfg.sanitize_return_to(Some("/dashboard/x")), "/dashboard/x");
        assert_eq!(cfg.sanitize_return_to(Some("//evil.com")), "/");
        assert_eq!(cfg.sanitize_return_to(Some("https://evil.com")), "/");
        assert_eq!(cfg.sanitize_return_to(None), "/");
    }

    #[test]
    fn authorize_url_contains_pkce_params() {
        let cfg = OidcConfig {
            issuer: "https://idp/realms/r".into(),
            client_id: "bff".into(),
            client_secret: "s".into(),
            redirect_uri: "https://bff/auth/callback".into(),
            allowed_return_paths: vec![],
        };
        let u = authorize_url(&cfg, "st", "nn", "ch");
        assert!(u.contains("code_challenge=ch"));
        assert!(u.contains("code_challenge_method=S256"));
        assert!(u.contains("state=st"));
        assert!(u.contains("nonce=nn"));
    }
}
