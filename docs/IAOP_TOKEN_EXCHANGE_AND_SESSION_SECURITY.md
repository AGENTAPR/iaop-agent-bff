# IAOP Token Exchange and Session Security Architecture
**Cryptographic Session Protection, Delegation, and Token Security Specification**

---

## 1. OAuth 2.0 Delegation & Audience Isolation

### Multi-Audience Edge Token Pattern
In IAOP, Keycloak issues user tokens containing the allowed audience set (`aud: ['iaop-agent-api', 'iaop-agency-api', ...]`) and client-specific roles mapped under `resource_access`.
- `iaop-agent-service` verifies `aud` contains `iaop-agent-api`.
- `iaop-agency-service` verifies `aud` contains `iaop-agency-api`.
- Domain tokens received with the wrong audience are rejected with `HTTP 401 Unauthorized`.

---

## 2. Server-Side Session Security

### 2.1 Storage & Encryption
Sessions are persisted in PostgreSQL table `user_sessions`:
- `id`: Opaque UUID v7 string identifier (never exposed in URL parameters).
- `data`: Binary ciphertext encrypted using AES-256-GCM.
- `created_at`, `last_seen`: Instant timestamps enforcing idle timeout (1 hour) and absolute timeout (12 hours).

### 2.2 Tampering & Fail-Closed Logic
When a session row is decrypted:
1. If the key is invalid or ciphertext authentication tag fails, decryption returns an error.
2. The session store treats failed decryption as an immediate session termination (`None`).
3. Corrupted or forged session cookies return `HTTP 401 Unauthorized` without error leakage.

---

## 3. Login Transaction Single-Use Atomicity

To prevent authorization code reuse and CSRF fixation:
1. `login_transactions` stores the pending login state and PKCE verifier.
2. During the callback, `atomic_claim_login_transaction` deletes the record atomically using `DELETE FROM login_transactions WHERE state = $1 RETURNING ...`.
3. If an attacker or duplicate redirect re-submits the same state, the delete returns zero rows and fails closed with `HTTP 401 Unauthorized`.
