# Demo User Security & End-to-End Test Matrix
**Verification Against Authoritative Accounts in `IAOP_DEV_DEMO_USERS.md`**

> **⚠ CORRECTION NOTICE (2026-11-15)** — The "CERTIFIED PRODUCTION READY" status
> below reflects the state at certification date (2026-11-14) and has been
> superseded by subsequent source audits that found unresolved defects
> (workspace fail-open, unverified JWT permission decoding, missing
> resource-level authorization, synthetic BFF data).
> See `IAOP_SECURITY_BFF_FINAL_REMEDIATION_REPORT.md` for the current,
> evidence-based verdict: **NOT READY**.
> Claims below are HISTORICAL and must not be treated as current.

---

## 1. Verified Keycloak Demo Accounts

All credentials match `IAOP_DEV_DEMO_USERS.md`. Passwords were synchronized and validated on live Instabox Keycloak (`iaop-keycloak-dev`).

| Username | Name | Realm Role | Client Roles | Allowed Operations | Forbidden Operations | Verification Result |
|---|---|---|---|---|---|---|
| `demo.agent.1` | Demo Agent One | `IAOP_AGENT` | `iaop-agent-api`: `[agent:read, agent:history:read]`<br>`iaop-agency-api`: `[agency:read, agency:assignment:read]` | View own profile (`GET /api/v1/me`), list own workspaces, view agency hierarchy | Create agent, change unit parent, terminate agent | **PASS** |
| `demo.agent.2` | Demo Agent Two | `IAOP_AGENT` | `iaop-agent-api`: `[agent:read, agent:history:read]`<br>`iaop-agency-api`: `[agency:read, agency:assignment:read]` | View own profile, switch permitted workspace | Access other agents' profile without admin privilege | **PASS** |
| `demo.agent.3` | Demo Agent Three | `IAOP_AGENT` | `iaop-agent-api`: `[agent:read, agent:history:read]`<br>`iaop-agency-api`: `[agency:read, agency:assignment:read]` | View own dashboard, view own team | Mutate agency hierarchy | **PASS** |
| `e2e-agent-admin` | E2E Platform Admin | `IAOP_PLATFORM_ADMIN` | Full administrative roles across all clients | Create agent, activate agent, suspend agent, terminate agent, assign leaders, change hierarchy | None (System Admin) | **PASS** |

---

## 2. Journey Test Assertions
1. **OIDC Code Flow with PKCE:** Confirmed for `demo.agent.1`. Returns valid JWT with 2047 bytes, verified RS256 signature, and active audiences.
2. **Identity Linkage:** Confirmed resolution for `issuer: https://8080-agq5lpbszgbva3ygddcbarv6ea.instabox.tech/realms/iaop-dev`.
3. **CSRF Enforcement:** Confirmed mutating endpoints reject requests without `x-csrf-token` with HTTP 403.
4. **Workspace Protection:** Confirmed arbitrary workspace IDs return HTTP 403 Forbidden.
