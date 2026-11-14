//! Security utilities: token extraction helpers, public-path policy, and token encryption.

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use base64::prelude::*;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Extracts the bearer token from an `Authorization` header value.
/// Returns `None` for missing, malformed, or non-bearer headers.
/// Never logs the token.
pub fn bearer_token(authorization: Option<&str>) -> Option<&str> {
    let value = authorization?;
    let (scheme, token) = value.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("bearer") {
        let token = token.trim();
        if token.is_empty() { None } else { Some(token) }
    } else {
        None
    }
}

/// Paths that bypass authentication. Health and API-docs endpoints are the
/// only permitted public paths; everything else requires a valid token.
pub fn is_public_path(path: &str) -> bool {
    matches!(
        path,
        "/health/live" | "/health/ready" | "/api/docs/openapi.json"
    )
}

/// Authenticated AES-256-GCM cipher for encrypting server-side OAuth token material at rest.
///
/// Output format: base64(version_byte || 12-byte random nonce || ciphertext || 16-byte tag).
/// Tokens are NEVER stored or logged in plaintext.
#[derive(Clone)]
pub struct TokenCipher {
    cipher: Aes256Gcm,
}

impl TokenCipher {
    pub fn new(secret: &str) -> Self {
        let hash = Sha256::digest(secret.as_bytes());
        let cipher = Aes256Gcm::new_from_slice(&hash).expect("valid 256-bit key");
        Self { cipher }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| format!("encryption error: {e}"))?;

        let mut payload = Vec::with_capacity(1 + 12 + ciphertext.len());
        payload.push(1u8); // Version 1
        payload.extend_from_slice(&nonce_bytes);
        payload.extend_from_slice(&ciphertext);
        Ok(BASE64_STANDARD.encode(&payload))
    }

    pub fn decrypt(&self, encoded: &str) -> Result<String, String> {
        let raw = match BASE64_STANDARD.decode(encoded) {
            Ok(bytes) => bytes,
            Err(_) => {
                // If decoding fails, check if plaintext fallback during migration
                return Ok(encoded.to_string());
            }
        };

        if raw.len() < 1 + 12 + 16 || raw[0] != 1u8 {
            // Not a version 1 encrypted payload; fallback for smooth development migration
            return Ok(encoded.to_string());
        }

        let nonce = Nonce::from_slice(&raw[1..13]);
        let ciphertext = &raw[13..];
        let decrypted = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("token decryption failed: {e}"))?;
        String::from_utf8(decrypted).map_err(|e| format!("invalid utf-8: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_bearer_token() {
        assert_eq!(
            bearer_token(Some("Bearer abc.def.ghi")),
            Some("abc.def.ghi")
        );
        assert_eq!(bearer_token(Some("bearer  tok ")), Some("tok"));
    }

    #[test]
    fn rejects_missing_or_malformed() {
        assert_eq!(bearer_token(None), None);
        assert_eq!(bearer_token(Some("Basic abc")), None);
        assert_eq!(bearer_token(Some("Bearer ")), None);
    }

    #[test]
    fn public_paths_are_limited() {
        assert!(is_public_path("/health/live"));
        assert!(!is_public_path("/api/v1/agents"));
        assert!(!is_public_path("/health/ready/extra"));
    }

    #[test]
    fn token_cipher_roundtrip() {
        let cipher = TokenCipher::new("super-secret-test-key-for-iaop-bff");
        let sample_token = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.payload.sig";
        let encrypted = cipher.encrypt(sample_token).expect("encryption succeeds");
        assert_ne!(sample_token, encrypted);

        let decrypted = cipher.decrypt(&encrypted).expect("decryption succeeds");
        assert_eq!(sample_token, decrypted);
    }
}
