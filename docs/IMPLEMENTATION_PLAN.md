# IAOP Agent BFF — Complete Implementation Plan
**Branch:** `development`  
**Date:** 2026-11-14

---

## Executive Summary

The `iaop-agent-bff` has a solid Rust/Axum foundation with a working OIDC auth flow (in-memory sessions only). The 134 agent-facing FE API operations are entirely absent. Only `iaop-agent-service` has real domain implementation; all other 15+ services are scaffold-only. 

**Implementation strategy:** Implement the BFF completely, integrate only against `iaop-agent-service` (the only real service), define typed adapters for all other services with honest dependency-unavailable responses, and mark all dependent E2E journeys BLOCKED_BY_DOMAIN_SERVICE.

---

## Stage 3: Foundation Hardening (In Progress — Subagent)

### Deliverables
- [x] FE response envelope `{data, meta}` / `{error, meta}` types
- [x] All 12 canonical error codes
- [x] PostgreSQL session store (`PgSessionStore`)
- [x] BFF database migrations (sessions, login_txns, preferences)
- [x] `AuthenticatedPrincipal` Axum extractor
- [x] Config additions (CORS origins, allowed return paths, agent service URL)
- [x] Unused `iaop-security` warning fix in infrastructure

### Non-breaking
- Existing `/health/*`, `/auth/*`, `/api/v1/system/info` routes preserved
- Existing passing tests preserved

---

## Stage 4: Identity & Personal API (Staged in Subagent)

### Routes
| Route | Method | Contract | Status |
|---|---|---|---|
| `/api/v1/me` | GET | IDENTITY-01 | Subagent |
| `/api/v1/me/workspaces` | GET | IDENTITY-02 | Subagent |
| `/api/v1/me/workspaces/select` | POST | IDENTITY-03 | Subagent |
| `/api/v1/me/preferences` | GET | IDENTITY-04 | Subagent |
| `/api/v1/me/preferences` | PATCH | IDENTITY-05 | Subagent |

### Integration
- Agent client: GET agent by OIDC subject
- Partial availability: returns `partial: true` when agent service unavailable

---

## Stage 5: Session Hardening

### Remaining work after Stage 3
- [ ] Token refresh race protection (advisory lock on session for refresh)
- [ ] Concurrent session policy (configurable max sessions per subject)
- [ ] Session housekeeping background task (clean expired sessions)
- [ ] CSRF middleware for state-changing `/api/v1/*` routes
- [ ] `SameSite=Strict` for sensitive cookies (upgrade from Lax)

---

## Stage 6: Today Dashboard

### Route
`GET /api/v1/me/today` — TODAY-01

### Architecture
```
Request → AuthPrincipal extractor
        → spawn concurrent tasks (bounded by tokio::time::timeout):
          ├── agent profile (agent-service)
          ├── eligibility summary (eligibility-service) [BLOCKED]
          ├── tasks (workflow-service) [BLOCKED]
          ├── notifications (notification-service) [BLOCKED]
          ├── agenda (workflow-service) [BLOCKED]
          └── performance insight (performance-service) [BLOCKED]
        → collect with partial availability
        → compose DashboardResponse
        → return FE envelope
```

### Response shape
```rust
pub struct TodayResponse {
    pub lifecycle_context: LifecycleContext,
    pub greeting: Greeting,
    pub eligibility: SectionResult<EligibilitySummary>,
    pub hero_action: Option<HeroAction>,
    pub tasks: SectionResult<TaskPage>,
    pub notifications: SectionResult<Vec<NotificationItem>>,
    pub agenda: SectionResult<Vec<AgendaItem>>,
    pub recent_activity: SectionResult<Vec<ActivityItem>>,
    pub performance_insight: SectionResult<Option<PerformanceInsight>>,
    pub as_of: DateTime<Utc>,
    pub partial: bool,
    pub unavailable_sections: Vec<String>,
}
```

### Priority logic
1. Critical restriction/blocker → hero action = "resolve restriction"
2. Urgent task (today due) → hero action = top urgent task
3. Time-sensitive follow-up → hero action = customer follow-up
4. Active workflow → hero action = continue workflow
5. Certification milestone → hero action = certification deadline
6. Default → "Start your day" greeting

### Partial availability
- Each section has independent `tokio::time::timeout(Duration::from_secs(3))`
- Failed/unavailable sections marked with `SectionResult::Unavailable { reason }`
- Dashboard returns 200 even with all sections unavailable

---

## Stage 7: Notifications & Search

### Routes
| Route | Method | Contract | Downstream |
|---|---|---|---|
| `/api/v1/me/notifications` | GET | UTILITIES-02 | notification-service [BLOCKED] |
| `/api/v1/me/notifications/{id}/read` | POST | UTILITIES-03 | notification-service [BLOCKED] |
| `/api/v1/search` | GET | UTILITIES-01 | Multiple [BLOCKED] |
| `/api/v1/support/tickets` | POST | UTILITIES-04 | Unowned |
| `/api/v1/support/tickets` | GET | UTILITIES-05 | Unowned |

### Notes
- Search: implement with agent-service query (by code/name) as first real source; all others BLOCKED
- Support tickets: ownership unresolved; expose a placeholder that returns `DEPENDENCY_UNAVAILABLE` with a tracking reference
- Notifications: return `DEPENDENCY_UNAVAILABLE` when notification-service is not available

---

## Stage 8: Work Items & Agenda

### Routes
| Route | Method | Contract |
|---|---|---|
| `/api/v1/me/tasks` | GET | TODAY-02 |
| `/api/v1/me/tasks/{taskId}` | PATCH | TODAY-03 |
| `/api/v1/me/agenda` | GET | TODAY-04 |

### Task model
```rust
pub struct TaskItem {
    pub task_id: String,
    pub origin_service: String,     // "workflow", "onboarding", etc.
    pub business_entity_type: String,
    pub business_entity_id: String,
    pub task_type: String,
    pub title: String,
    pub priority: TaskPriority,
    pub due_at: Option<DateTime<Utc>>,
    pub status: TaskStatus,
    pub allowed_actions: Vec<String>,
    pub deep_link: Option<String>,
    pub version: u64,
    pub as_of: DateTime<Utc>,
}
```

---

## Stage 9: Domain API Composition (agent-service integrated, others BLOCKED)

### Recruitment (RECRUITMENT-01 through 08)
| Op | Route | Downstream | Status |
|---|---|---|---|
| RECRUITMENT-01 | GET `/api/v1/recruitment/candidates` | recruitment-service | BLOCKED |
| RECRUITMENT-02 | POST `/api/v1/recruitment/candidates` | recruitment-service | BLOCKED |
| RECRUITMENT-03 | GET `/api/v1/recruitment/candidates/{id}` | recruitment-service | BLOCKED |
| RECRUITMENT-04 | GET `/api/v1/recruitment/my-referrals` | recruitment-service | BLOCKED |

### Onboarding (ONBOARDING-01 through 08)
All BLOCKED by onboarding-service being scaffold-only.

### Learning (LEARNING-01 through 09)
All BLOCKED by learning-service.

### Certification (CERTIFICATION-01 through 10)
All BLOCKED by certification-service.

### Agency (AGENCY-01 through 08)
Partially implemented using agent-service (read own agent record). Agency structure BLOCKED.

### Customers, Sales, Policies, Claims (all BLOCKED)
Domain services scaffold-only. Routes implemented with `DEPENDENCY_UNAVAILABLE` responses.

### Earnings (EARNINGS-01 through 07)
BLOCKED by commission-service.

### Performance (PERFORMANCE-01 through 03)
BLOCKED by performance-service.

---

## Stage 10: Operations & Security Boundaries

### Agent-facing operations (permitted)
- `GET /api/v1/me/offboarding-status` — OFFBOARDING-04: view own offboarding status
- `GET /api/v1/me/restrictions` — COMPLIANCE-01 agent view: view own permitted restrictions
- `GET /api/v1/operations/{operationId}` — view operation status for agent-initiated ops

### Excluded from Agent BFF (delegated to Ops BFF)
- OPERATIONS-01 through 06 (admin work queues)
- AUDIT-01 (privileged audit)
- INTEGRATION-01 through 03 (integration retries)
- COMPLIANCE-02 through 03 (compliance decisions)
- OFFBOARDING-01 through 03 (privileged case management)

---

## Stage 11: Security Test Implementation

### Critical security tests to implement
```rust
// Negative tests (must all FAIL correctly, returning the right error)
1. unauthenticated_request_returns_401
2. expired_session_returns_401  
3. revoked_session_returns_401
4. invalid_csrf_on_logout_returns_403
5. invalid_csrf_on_workspace_select_returns_403
6. invalid_csrf_on_preferences_patch_returns_403
7. invalid_oidc_state_returns_400
8. cross_session_resource_access_returns_403
9. wrong_issuer_id_token_rejected
10. expired_id_token_rejected
11. replay_login_state_rejected
12. open_redirect_sanitized_to_allowlist
13. idempotency_key_reused_with_different_payload_returns_422
```

---

## Stage 12: Performance & Load Baselines

### Targets (initial — not production SLAs)
- `/api/v1/me`: p95 < 100ms with no agent service call
- `/api/v1/me`: p95 < 500ms with agent service call
- `/api/v1/me/today`: p95 < 3s with partial availability
- Session lookup: p95 < 10ms (in-memory), < 50ms (PostgreSQL)

### Tools
- Use `criterion` for unit benchmarks
- Use `k6` or similar for HTTP load tests (not committed to repo)

---

## Stage 13: Deployment & Production Hardening

### Dockerfile verification
- Multi-stage build
- Non-root execution
- Health check endpoints
- Graceful shutdown (already implemented via tokio signal handling)

### Production checklist
- PostgreSQL session store wired (requires DATABASE_URL)
- OIDC configured (requires OIDC_* env vars)
- CORS origins configured (CORS_ALLOWED_ORIGINS)
- Session encryption key configured (future: SESSION_ENCRYPTION_KEY)
- Database migrations applied on startup

---

## Blocker Registry

| Blocker | Affected Contracts | Required Work |
|---|---|---|
| `iaop-recruitment-service` scaffold-only | RECRUITMENT-01..08 | Implement recruitment domain |
| `iaop-onboarding-service` scaffold-only | ONBOARDING-01..08 | Implement onboarding domain |
| `iaop-learning-service` scaffold-only | LEARNING-01..09 | Implement learning domain |
| `iaop-certification-service` scaffold-only | CERTIFICATION-01..10 | Implement certification domain |
| `iaop-eligibility-service` scaffold-only | Eligibility summary | Implement eligibility |
| `iaop-sales-service` scaffold-only | SALES-01..13 | Implement sales domain |
| `iaop-portfolio-service` scaffold-only | POLICIES-01..08 | Implement portfolio domain |
| `iaop-commission-service` scaffold-only | EARNINGS-01..07 | Implement commission domain |
| `iaop-performance-service` scaffold-only | PERFORMANCE-01..03 | Implement performance domain |
| `iaop-notification-service` scaffold-only | UTILITIES-02..03 | Implement notification domain |
| `iaop-workflow-service` scaffold-only | Tasks, Agenda | Implement workflow |
| Support ticket owner unresolved | UTILITIES-04..05 | Architecture decision required |
| Customer/CRM owner unresolved | CUSTOMERS-01..14 | Architecture decision required |
| Claim service missing | CLAIMS-01..08 | Implement claims service |
| `iaop-agency-service` partial | AGENCY-04..08 | Complete agency domain |

---

## Production Readiness Checklist

| Item | Current | Target |
|---|---|---|
| Build succeeds | ✅ | ✅ |
| All existing tests pass | ✅ | ✅ |
| PostgreSQL session store | ❌ | ✅ |
| FE response envelope | ❌ | ✅ |
| `/api/v1/me` implemented | ❌ | ✅ |
| `/api/v1/me/today` dashboard | ❌ | ✅ |
| Security negative tests | ❌ | ✅ |
| CSRF protection on API routes | ❌ | ✅ |
| Docker build verified | 🔄 | ✅ |
| Agent service integration working | ❌ | ✅ |
| All other domain integrations | ❌ | BLOCKED |
| E2E: Login → Session → Profile | ❌ | ✅ |
| E2E: Workspace selection | ❌ | ✅ |
| E2E: All 134 FE ops end-to-end | ❌ | Partially BLOCKED |

**BFF Implementation Complete** ≠ **IAOP Ecosystem Production-Ready**

The BFF implementation can be complete and verified while dependent domain services remain scaffold-only. Those are separate engineering tasks.
