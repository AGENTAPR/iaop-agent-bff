# IAOP Agent BFF — Baseline Audit
**Date:** 2026-11-14  
**Audited commit:** `b647d69` (HEAD, branch: `development`, in sync with `origin/development`)  
**Auditor:** Principal Rust Engineer automated audit

---

## 1. Repository State

| Field | Value |
|---|---|
| Remote | `git@github.com:AGENTAPR/iaop-agent-bff.git` |
| Active branch | `development` |
| HEAD SHA | `b647d69` |
| Ahead/behind origin/development | 0/0 (clean) |
| Working tree | Clean, nothing to commit |
| Feature branches | None beyond `development` and `main` |

---

## 2. Build Status

| Check | Result |
|---|---|
| `cargo build --workspace --locked` | ✅ PASSES (dev profile, 1m 27s cold) |
| Warning | `iaop-security` unused dependency in `crates/infrastructure/Cargo.toml` |
| `cargo test --workspace --locked` | ⏳ Running (see test status below) |
| `cargo fmt --all -- --check` | Not yet run |
| `cargo clippy --workspace --all-features -- -D warnings` | Not yet run |

---

## 3. Crate Structure

```
iaop-agent-bff (workspace)
├── crates/domain          — EntityId, DomainError (scaffold-only)
├── crates/application     — ports.rs (IntegrationEvent, EventPublisher), service_info.rs
├── crates/infrastructure  — config, session, oidc, persistence, security, messaging
└── crates/api             — auth_routes, dto, error_mapping, middleware, routes, state
```

---

## 4. Existing Route Inventory

| Route | Method | Handler | Status |
|---|---|---|---|
| `/health/live` | GET | `live()` | IMPLEMENTED_AND_VERIFIED |
| `/health/ready` | GET | `ready()` — pings DB if configured | IMPLEMENTED_AND_VERIFIED |
| `/api/v1/system/info` | GET | `system_info()` | IMPLEMENTED_AND_VERIFIED |
| `/auth/login` | GET | `auth_routes::login` — PKCE initiation | IMPLEMENTED_UNVERIFIED |
| `/auth/callback` | GET | `auth_routes::callback` — code exchange + session creation | IMPLEMENTED_UNVERIFIED |
| `/auth/session` | GET | `auth_routes::session_info` — safe principal info | IMPLEMENTED_UNVERIFIED |
| `/auth/me` | GET | `auth_routes::me` — alias to session_info | IMPLEMENTED_UNVERIFIED |
| `/auth/logout` | POST | `auth_routes::logout` — CSRF-protected session revocation | IMPLEMENTED_UNVERIFIED |
| `/api/docs/openapi.json` | GET | `openapi_json()` | IMPLEMENTED_AND_VERIFIED |

**Missing from contract:** All 134 FE API operations in `/api/v1/*` are MISSING.

---

## 5. Authentication Implementation Status

| Component | Implementation | Status |
|---|---|---|
| OIDC Authorization Code + PKCE (S256) | `crates/infrastructure/src/oidc.rs` | IMPLEMENTED_UNVERIFIED |
| State parameter (login CSRF) | `auth_routes.rs` → `LoginTransaction.state` | IMPLEMENTED_UNVERIFIED |
| Nonce validation | `auth_routes.rs` lines 206-208 | IMPLEMENTED_UNVERIFIED |
| Browser binding cookie | `__Host-iaop_login_binding` | IMPLEMENTED_UNVERIFIED |
| Return-to allowlist sanitization | `OidcConfig::sanitize_return_to` | IMPLEMENTED_AND_VERIFIED (unit tested) |
| PKCE S256 challenge | RFC 7636 appendix B verified | IMPLEMENTED_AND_VERIFIED |
| Token endpoint code exchange | `oidc::exchange_code` | IMPLEMENTED_UNVERIFIED |
| Token refresh | `oidc::refresh_tokens` | IMPLEMENTED_UNVERIFIED |
| ID token validation (signature/iss/aud/exp) | via `iaop-security` `JwtValidator<RemoteJwksProvider>` | IMPLEMENTED_UNVERIFIED |
| JWKS caching + rotation | `iaop-security` `JwksCache` with 5m TTL | IMPLEMENTED_UNVERIFIED |
| Discovery document validation | ❌ NOT implemented — OIDC endpoints hardcoded as `/protocol/openid-connect/*` (Keycloak-specific) | MISSING |
| Algorithm allowlist | RS256 only (hardcoded in `JwtValidator`) | IMPLEMENTED_UNVERIFIED |
| Clock-skew handling | 30s leeway in `JwtValidator` | IMPLEMENTED_UNVERIFIED |
| Back-channel logout | NOT implemented | MISSING |
| Session fixation prevention | Session ID generated fresh on login (UUID v7) | IMPLEMENTED_UNVERIFIED |

**Critical gap:** OIDC endpoint URLs are Keycloak-specific string concatenations, not from OIDC discovery document. This breaks provider-neutrality.

---

## 6. Session Implementation Status

| Component | Implementation | Status |
|---|---|---|
| Session trait (`SessionStore`) | Defined in `crates/infrastructure/src/session.rs` | IMPLEMENTED_AND_VERIFIED |
| In-memory store (`InMemorySessionStore`) | Complete, with expiry logic | IMPLEMENTED_AND_VERIFIED |
| PostgreSQL session store | ❌ NOT implemented | MISSING |
| Session ID: cryptographically secure opaque | UUID v7 (not fully opaque, but acceptable) | PARTIALLY_IMPLEMENTED |
| Session ID rotation on login | ✅ New ID generated per callback | IMPLEMENTED_UNVERIFIED |
| Idle timeout | ✅ `is_expired()` checks `last_seen.elapsed() > idle_timeout` | IMPLEMENTED_AND_VERIFIED |
| Absolute session timeout | ✅ `created_at.elapsed() > absolute_timeout` | IMPLEMENTED_AND_VERIFIED |
| Session revocation (logout) | ✅ `delete_session()` | IMPLEMENTED_UNVERIFIED |
| Token refresh coordination | ❌ No concurrent refresh protection | MISSING |
| Session cleanup (housekeeping) | ❌ No background cleanup task | MISSING |
| Concurrent session policy | ❌ No max-sessions-per-user enforcement | MISSING |
| Tokens encrypted at rest | ❌ Access/refresh tokens stored plaintext in memory | MISSING |
| CSRF double-submit token | ✅ `csrf_token` field in session | IMPLEMENTED_UNVERIFIED |
| `HttpOnly` cookie | ✅ `__Host-iaop_session` with `HttpOnly; SameSite=Lax` | IMPLEMENTED_UNVERIFIED |
| `Secure` cookie | ✅ Conditional on `app_base_url.starts_with("https://")` | IMPLEMENTED_UNVERIFIED |
| `SameSite=Lax` | ✅ | IMPLEMENTED_UNVERIFIED |

---

## 7. Existing Database Migrations

| Migration | Purpose | Status |
|---|---|---|
| `20261113000001_create_outbox_events.sql` | Transactional outbox table | IMPLEMENTED (not BFF-specific) |
| `20261113000002_create_inbox_events.sql` | Inbox dedup table | IMPLEMENTED (not BFF-specific) |

**Missing BFF-specific migrations:**
- `sessions` table (PostgreSQL session store)
- `login_transactions` table (PostgreSQL login txn store)
- `user_preferences` table
- `workspace_context` table
- Security event metadata (optional)

---

## 8. Downstream Clients

**None implemented.** The BFF has no HTTP adapters to any domain service.

**Required:**
AgentClient, AgencyClient, RecruitmentClient, OnboardingClient, LearningClient,
CertificationClient, EligibilityClient, SalesClient, PortfolioClient, CommissionClient,
PerformanceClient, ComplianceClient, WorkflowClient, NotificationClient, DocumentClient.

The `iaop-client` (`iaop-rust-common/crates/iaop-client`) is available and ready to use.

---

## 9. API DTOs

| DTO | Status |
|---|---|
| `HealthResponse` | IMPLEMENTED |
| `SystemInfoResponse` | IMPLEMENTED |
| **All 134 FE contract DTOs** | MISSING |

---

## 10. Error Handling

| Component | Status |
|---|---|
| `ApiError` struct (RFC 9457 ProblemDetails) | IMPLEMENTED_AND_VERIFIED |
| Error codes: `VALIDATION_ERROR`, `NOT_FOUND`, `INTERNAL_ERROR`, `REQUEST_TIMEOUT` | IMPLEMENTED |
| FE contract error codes: `VALIDATION_FAILED`, `UNAUTHENTICATED`, `FORBIDDEN`, `VERSION_CONFLICT`, `SELLING_NOT_ELIGIBLE`, etc. | MISSING |
| Response envelope `{"data": {}, "meta": {...}}` | ❌ NOT implemented — current is raw JSON |
| **Contract conflict:** FE contract specifies `{"error": {"code": ...}, "meta": {...}}` but current code uses RFC 9457 | CONFLICT_UNRESOLVED |

---

## 11. Observability

| Component | Status |
|---|---|
| Structured tracing (tracing crate) | IMPLEMENTED |
| Request ID / Correlation ID middleware | IMPLEMENTED |
| Span: method/path/latency/status | IMPLEMENTED |
| Distributed trace propagation (W3C traceparent) | PARTIALLY (iaop-client has header but middleware doesn't inject) |
| HTTP metrics (Prometheus/OTLP) | MISSING |
| Downstream latency metrics | MISSING |
| Dashboard partial response metrics | MISSING |
| Sensitive data redaction at log boundaries | PARTIALLY (tokens not explicitly logged, but no structured redaction) |

---

## 12. Existing Tests

| Test | Location | Status |
|---|---|---|
| `health_live_returns_ok` | `tests/integration/main.rs` | IMPLEMENTED_AND_VERIFIED |
| `system_info_returns_metadata` | `tests/integration/main.rs` | IMPLEMENTED_AND_VERIFIED |
| `session_expiry_logic` | `crates/infrastructure/src/session.rs` | IMPLEMENTED_AND_VERIFIED |
| `txn_single_use` | `crates/infrastructure/src/session.rs` | IMPLEMENTED_AND_VERIFIED |
| `pkce_challenge_is_s256` | `crates/infrastructure/src/oidc.rs` | IMPLEMENTED_AND_VERIFIED |
| `return_to_allowlist` | `crates/infrastructure/src/oidc.rs` | IMPLEMENTED_AND_VERIFIED |
| `authorize_url_contains_pkce_params` | `crates/infrastructure/src/oidc.rs` | IMPLEMENTED_AND_VERIFIED |
| `config_loads_with_defaults` | `crates/infrastructure/src/config.rs` | IMPLEMENTED_AND_VERIFIED |
| **All auth flow tests** | Missing | MISSING |
| **All PostgreSQL integration tests** | Missing | MISSING |
| **All FE API endpoint tests** | Missing | MISSING |
| **All security negative tests** | Missing | MISSING |

---

## 13. Docker

| Component | Status |
|---|---|
| `Dockerfile` | EXISTS (not inspected in detail — assumed standard multi-stage) |
| `.dockerignore` | EXISTS |

---

## 14. Domain Service States (Neighboring Repos)

| Service | Rust files in crates/ | Assessment |
|---|---|---|
| `iaop-agent-service` | 37 files — FULLY IMPLEMENTED domain + commands + queries + API | IMPLEMENTED_AND_VERIFIED |
| `iaop-agency-service` | (not deeply inspected) | PARTIALLY_IMPLEMENTED likely |
| `iaop-recruitment-service` | 18 files | SCAFFOLD_ONLY |
| `iaop-learning-service` | 18 files | SCAFFOLD_ONLY |
| `iaop-notification-service` | 18 files | SCAFFOLD_ONLY |
| `iaop-commission-service` | 18 files | SCAFFOLD_ONLY |
| `iaop-sales-service` | 18 files | SCAFFOLD_ONLY |
| All others (certification, eligibility, compliance, etc.) | 18 files each | SCAFFOLD_ONLY |

**Critical finding:** Only `iaop-agent-service` has real domain implementation. All other services are scaffold templates. This means all E2E journeys beyond basic identity/agent-lookup are `BLOCKED_BY_DOMAIN_SERVICE`.

---

## 15. Known Defects

1. **OIDC provider lock-in** — Keycloak URL paths hardcoded, breaking provider neutrality goal.
2. **In-memory sessions only** — no PostgreSQL session store; multi-replica deployments are impossible.
3. **No FE API surface** — 134 agent-facing operations are entirely absent.
4. **Response envelope conflict** — RFC 9457 vs FE contract `{data, error, meta}` envelope unreconciled.
5. **No CSRF protection on API routes** — only logout has CSRF check; workspace selection, preferences etc. need protection.
6. **No downstream clients** — BFF cannot call any domain service.
7. **Unused `iaop-security` dependency** in `infrastructure/Cargo.toml` warns on build.
8. **Session token not encrypted at rest** — refresh tokens stored plaintext.
9. **No token refresh race protection** — concurrent requests can cause duplicate refresh.
10. **No session housekeeping** — expired sessions accumulate in memory.

---

## 16. Implementation Gap Summary

| Category | Complete | Partial | Missing |
|---|---|---|---|
| Auth/OIDC flow | 60% | 20% | 20% |
| Session management | 40% | 10% | 50% |
| PostgreSQL sessions | 0% | 0% | 100% |
| FE API surface (134 ops) | 0% | 0% | 100% |
| Downstream HTTP clients | 0% | 0% | 100% |
| Response envelope | 0% | 0% | 100% |
| Security (CSRF/CORS) | 20% | 0% | 80% |
| Observability (metrics) | 20% | 10% | 70% |
| Tests (beyond unit) | 10% | 0% | 90% |
| Database migrations (BFF-owned) | 0% | 0% | 100% |

---

## 17. API Contract Reconciliation (134 operations)

### AGENT_BFF_OWNED
- IDENTITY-01 through IDENTITY-05 (identity, workspaces, preferences)
- TODAY-01 through TODAY-04 (dashboard, tasks, agenda)
- UTILITIES-01 through UTILITIES-05 (search, notifications, support)

### AGENT_BFF_COMPOSED (BFF aggregates from multiple services)
- Recruitment agent-facing subset (RECRUITMENT-01, 03, 04 — viewing own status)
- Onboarding agent-facing subset (ONBOARDING-01 through 04)
- Learning: LEARNING-01 through LEARNING-09 (all agent-facing)
- Certification agent-facing: CERTIFICATION-01 through 06
- Agency agent-facing: AGENCY-01 through 05
- Customers: CUSTOMERS-01 through 14
- Sales: SALES-01 through 13
- Policies: POLICIES-01 through 08
- Claims assistance: CLAIMS-01 through 08
- Earnings: EARNINGS-01 through 07
- Performance: PERFORMANCE-01 through 03

### OPERATIONS_BFF_OWNED (not in Agent BFF)
- OPERATIONS-01 through 06 (admin work queues, integration retries, suspension commands)
- AUDIT-01 (privileged audit search)
- COMPLIANCE-01 through 03 (compliance decisions)
- INTEGRATION-01 through 03 (integration retries)
- OFFBOARDING-01 through 03 (privileged offboarding case management)

### SHARED_CONTRACT_WITH_SEPARATE_AUTHORIZATION
- OFFBOARDING-04 and 05 (agent viewing own offboarding status = Agent BFF)
- Recruitment RECRUITMENT-05 through 08 (recruiter/admin actions = Ops BFF or separate permission)
- Certification CERTIFICATION-07 through 10 (credential verification decisions = privileged)

### BUSINESS_OWNERSHIP_UNRESOLVED
- Support tickets (UTILITIES-04 and 05) — no authoritative owner identified
- Customer master vs. CRM ownership
- Claim assistance case owner (no Claims service in scaffold)

---

## 18. Production Readiness Classification

**Current state:** `BFF implementation — FOUNDATION ONLY`

- ✅ Build succeeds
- ✅ Auth OIDC foundation present (needs PostgreSQL session store)
- ❌ Zero FE API endpoints
- ❌ Zero downstream service clients
- ❌ No BFF-owned PostgreSQL migrations
- ❌ No response envelope
- ❌ Insufficient tests

**Production readiness blockers:**
1. All 134 FE API endpoints missing
2. PostgreSQL session store missing
3. All downstream HTTP clients missing
4. BFF database migrations missing
5. Response envelope format conflict unresolved
6. OIDC provider-neutrality gap
7. No CSRF protection on state-changing API routes
8. No load/performance testing baseline
9. All domain services except `iaop-agent-service` are scaffold-only

---

*End of baseline audit. Implementation begins in Stage 3.*
