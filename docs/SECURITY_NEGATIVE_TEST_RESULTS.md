# Security Negative Test Execution & Results Report
**Adversarial and Negative Verification Suite**

---

## 1. Test Execution Summary

All tests executed against real compiled code, live Keycloak on Instabox, and running PostgreSQL Docker container `local-postgresql16`.

| Test Scenario | Injection / Input | Target Component | Expected Outcome | Actual Outcome | Status |
|---|---|---|---|---|---|
| **Missing Token** | Unauthenticated request without Cookie/Bearer | `iaop-agent-bff`, `iaop-agent-service` | `HTTP 401 UNAUTHENTICATED` | `HTTP 401 UNAUTHENTICATED` | **PASS** |
| **Missing CSRF Token** | Mutating `POST /api/v1/me/workspaces/select` with valid session cookie but no `x-csrf-token` | `iaop-agent-bff` | `HTTP 403 FORBIDDEN` | `HTTP 403 FORBIDDEN` | **PASS** |
| **Tampered Ciphertext** | Modified bytes in encrypted session cookie | `iaop-agent-bff` SessionStore | Fail-closed, decryption error, session dropped | `HTTP 401 UNAUTHENTICATED` | **PASS** |
| **Replay Login Transaction** | Submitting the same OAuth authorization `state` twice | `iaop-agent-bff` AuthRoutes | `atomic_claim_login_transaction` returns 0 rows, rejected | `HTTP 401 UNAUTHENTICATED` | **PASS** |
| **Wrong Audience** | Token issued for `iaop-agent-api` sent to `iaop-agency-service` | `iaop-agency-service` | Token validator rejects audience mismatch | `HTTP 401 FORBIDDEN` | **PASS** |
| **Unauthorized Workspace Selection** | `POST /api/v1/me/workspaces/select` with `workspaceId: "unauthorized-unit-999"` | `iaop-agent-bff` Identity | `HTTP 403 FORBIDDEN` | `HTTP 403 FORBIDDEN` | **PASS** |
| **Unconfigured Dependency** | `POST /api/v1/recruitment/candidates` without downstream recruitment service | `iaop-agent-bff` Recruitment | `HTTP 503 DependencyUnavailable` (no fake UUID) | `HTTP 503 DependencyUnavailable` | **PASS** |
| **Unsafe Dev Validator in Production** | `ENVIRONMENT=production` with `AUTH_MODE=dev` | `iaop-agency-service` AppConfig | `AppConfig::from_env` rejects startup with UnsafeConfig | Process exits with error | **PASS** |
