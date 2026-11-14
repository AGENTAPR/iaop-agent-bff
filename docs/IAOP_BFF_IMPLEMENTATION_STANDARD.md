# IAOP Backend for Frontend (BFF) Implementation Standard
**Specification for Secure, Performant User-Journey API Composition**

---

## 1. Role and Core Boundaries
The IAOP Agent BFF (`iaop-agent-bff`) acts as the dedicated frontend-facing orchestration service for the AGENA platform.

### Strict Architectural Boundaries
1. **No Domain Authority:** The BFF is not a domain database. It does not generate authoritative agent IDs, does not make compliance decisions, and does not store business entities.
2. **Session Ownership:** The BFF is the sole component that manages browser sessions, session cookies, and PKCE-based OIDC flows.
3. **Upstream Token Delegation:** The BFF propagates authenticated user credentials downstream via audience-scoped Bearer tokens, appending tracing context (`X-Request-ID`, `X-Correlation-ID`).

---

## 2. Session Management & Cookie Hardening

### 2.1 Cookie Configuration
| Directive | Production Value | Development Value |
|---|---|---|
| Cookie Name | `__Host-iaop_session_secure` | `iaop_session` |
| `HttpOnly` | `true` | `true` |
| `Secure` | `true` (HTTPS only) | `false` (HTTP allowed) |
| `SameSite` | `Lax` | `Lax` |
| `Path` | `/` | `/` |

### 2.2 CSRF Protection Model
State-changing HTTP operations (`POST`, `PUT`, `PATCH`, `DELETE`) require a double-submit CSRF verification pattern:
- A cryptographically random `csrf_token` is generated on session creation.
- The client reads the `csrfToken` field from the initial authentication envelope or profile response.
- Mutating requests must include the header `x-csrf-token: <token>`.
- The `AuthenticatedPrincipal` extractor rejects mismatches with HTTP 403 Forbidden.

### 2.3 Session Data Encryption
All session records stored in the `user_sessions` PostgreSQL table are encrypted with AES-256-GCM. 
- Encryption Key: 256-bit key supplied via `SESSIONS_ENCRYPTION_KEY` or `BFF_SESSION_KEY`.
- Nonce: Cryptographically fresh 96-bit IV generated per write.
- Integrity: Authenticated ciphertext prevents tampering; unauthenticated or corrupted ciphertext fails closed immediately, invalidating the session.

---

## 3. Upstream HTTP Client Architecture

### Client Reuse & Pooling
1. A single shared `reqwest::Client` instance is instantiated per downstream domain adapter during BFF startup.
2. Connection pooling maintains keep-alive sockets to reduce TCP and TLS handshake overhead.
3. Bounded Request Deadlines:
   - Read operations: Default timeout of 3.0 seconds.
   - Mutating commands: Default timeout of 5.0 seconds.
   - Aggregate dashboards: 3.0 second Tokio timeout per independent downstream call.
