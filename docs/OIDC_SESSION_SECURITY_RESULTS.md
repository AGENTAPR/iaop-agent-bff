# IAOP Agent BFF — OIDC Session Security & Cryptographic Audit Report

**Component**: `AGENTAPR/iaop-agent-bff`  
**Security Classification**: CONFIDENTIAL / INFRASTRUCTURE SECURITY AUDIT  
**Auditor**: Application Security & Cryptographic Architect  
**Status**: **VERIFIED & SECURE (PASS)**

---

## 1. Executive Summary

During the production readiness remediation of `iaop-agent-bff`, two critical security defects were identified and eliminated:
1. **P0 Vulnerability — Plaintext Encryption Fallback**: The session token cipher silently returned raw strings if decrypting AES-256-GCM failed or if the `v1:` prefix was absent. This created a catastrophic fail-open vulnerability where tampered or plain tokens would be treated as valid.
2. **P0 Concurrency Defect — Thread-Blocking DB Calls in Async Context**: `PgSessionStore` used `tokio::task::block_in_place` wrapping `block_on` to call asynchronous SQLx database queries from synchronous trait methods, risking runtime worker thread starvation and thread pool exhaustion under production traffic.

Both defects have been remediated, verified with automated unit/integration tests, and proven against a live PostgreSQL 16 database.

---

## 2. Token Cipher Cryptographic Remediation

### 2.1 Implementation (`crates/infrastructure/src/security.rs`)
The cipher now implements strict fail-closed AES-256-GCM envelope encryption:

```rust
pub struct TokenCipher {
    cipher: Aes256Gcm,
}

impl TokenCipher {
    pub fn encrypt(&self, plaintext: &str) -> Result<String, SecurityError> {
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self.cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| SecurityError::Internal(e.to_string()))?;

        let mut payload = nonce_bytes.to_vec();
        payload.extend_from_slice(&ciphertext);
        Ok(format!("v1:{}", STANDARD.encode(&payload)))
    }

    pub fn decrypt(&self, ciphertext: &str) -> Result<String, SecurityError> {
        let payload_b64 = ciphertext
            .strip_prefix("v1:")
            .ok_or_else(|| SecurityError::InvalidToken("unsupported ciphertext format"))?;

        let raw = STANDARD.decode(payload_b64)
            .map_err(|_| SecurityError::InvalidToken("invalid base64 encoding"))?;

        if raw.len() < 12 + 16 {
            return Err(SecurityError::InvalidToken("ciphertext too short"));
        }

        let (nonce_bytes, ct_bytes) = raw.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let pt_bytes = self.cipher
            .decrypt(nonce, ct_bytes)
            .map_err(|_| SecurityError::InvalidToken("decryption failed or data tampered"))?;

        String::from_utf8(pt_bytes)
            .map_err(|_| SecurityError::InvalidToken("invalid utf-8 payload"))
    }
}
```

### 2.2 Security Tests Executed

| Test Name | Attack Vector Tested | Expected Result | Actual Result | Status |
|---|---|---|---|---|
| `token_cipher_roundtrip` | Valid encrypt/decrypt cycle | Decrypted plaintext equals original | Exact match | **PASS** |
| `rejects_plaintext_fallback` | Attacker inputs unencrypted token (`"my-secret"`) | `Err(InvalidToken("unsupported ciphertext format"))` | Rejected fail-closed | **PASS** |
| `rejects_invalid_base64` | Corrupted base64 characters (`"v1:!!@@##$$"`) | `Err(InvalidToken("invalid base64 encoding"))` | Rejected fail-closed | **PASS** |
| `rejects_unsupported_version` | Unknown format prefix (`"v2:dGVzdA=="`) | `Err(InvalidToken("unsupported ciphertext format"))` | Rejected fail-closed | **PASS** |
| `rejects_tampered_ciphertext` | Single bit flip in encrypted bytes | `Err(InvalidToken("decryption failed or data tampered"))` | Authentication tag failure | **PASS** |

---

## 3. Asynchronous Session Storage Architecture

### 3.1 Async Trait Migration
The `SessionStore` trait (`crates/infrastructure/src/session.rs`) was upgraded to native asynchronous execution:

```rust
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn save_session(&self, session: &Session) -> Result<(), SessionError>;
    async fn get_session(&self, session_id: &str) -> Result<Option<Session>, SessionError>;
    async fn delete_session(&self, session_id: &str) -> Result<(), SessionError>;
    async fn extend_ttl(&self, session_id: &str, new_expires_at: OffsetDateTime) -> Result<(), SessionError>;
    async fn save_login_transaction(&self, txn: &LoginTransaction) -> Result<(), SessionError>;
    async fn consume_login_transaction(&self, state: &str) -> Result<Option<LoginTransaction>, SessionError>;
}
```

### 3.2 Real PostgreSQL Database Integration Tests
Live tests executed against database `iaop_agent_bff` on container `local-postgresql16`:

```text
running 2 tests
test atomic_single_use_login_transactions_in_real_database ... ok
test encrypted_session_persistence_and_fail_closed_on_tampering ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
```

1. **`encrypted_session_persistence_and_fail_closed_on_tampering`**:
   - Persists a session with tokens encrypted by `TokenCipher`.
   - Reads back and asserts valid decrypted session.
   - Updates database row directly with tampered ciphertext.
   - Verifies session load fails with `SessionError::StorageError` and does NOT return corrupt or partial data.

2. **`atomic_single_use_login_transactions_in_real_database`**:
   - Creates a pending OIDC `LoginTransaction` with PKCE code verifier and state.
   - Calls `consume_login_transaction` -> successfully retrieves transaction and atomically deletes it in a single atomic SQL transaction.
   - Second call to `consume_login_transaction` returns `Ok(None)`, preventing replay attacks.

---

## 4. End-to-End Session Security Features

- **CSRF Token Validation**: Every state-changing API endpoint (`POST`, `PUT`, `DELETE`) requires `X-CSRF-Token` header matching the active session's CSRF token.
  - Missing header: HTTP 403 Forbidden.
  - Invalid token: HTTP 403 Forbidden.
  - Valid token: HTTP 200 OK.
  - Verified by tests `mutating_request_without_csrf_returns_403` and `mutating_request_with_valid_csrf_succeeds`.
- **Session Expiry**: Strict evaluation of `expires_at` against UTC timestamps. Expired sessions are pruned and rejected.
- **Frontend-Safe Cookie Options**: Session cookie issued with `HttpOnly`, `SameSite=Lax`, and `Secure` flags.

---

## 5. Conclusion
The BFF authentication and session layer adheres strictly to zero-trust principles: tokens are cryptographically protected at rest, session operations never block executor threads, replay attacks are blocked by atomic single-use state consumption, and tampering causes immediate fail-closed rejection.
