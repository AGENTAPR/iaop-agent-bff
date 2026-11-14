# IAOP Revoked and Inactive User Security Policy
**Risk-Based Freshness, Revocation Boundaries, and Fail-Closed Policy**

---

## 1. Multi-Tier Status Hierarchy

An agent's operational capability depends on four distinct state layers:

```
[Keycloak Identity Status: ENABLED / DISABLED]
      ↓
[Agent Registry Lifecycle Status: DRAFT / PENDING / ACTIVE / SUSPENDED / TERMINATED]
      ↓
[Agency Assignment Status: ACTIVE / ENDED / CANCELLED]
      ↓
[Authoritative Selling Eligibility: ELIGIBLE / INELIGIBLE (Licenses, Sanctions, CPD)]
```

---

## 2. Risk-Based Freshness Enforcement

| Operation Risk Tier | Examples | Freshness Guarantee | Validation Mechanism |
|---|---|---|---|
| **Low-Risk Reads** | View agent code, view personal theme, view FAQ | Bounded Cache (≤ 5 min) | Cached JWT / local JWKS verification |
| **Normal Operations** | View team list, view personal profile, dashboard | Dynamic Domain Query | Real-time query to domain service with active session |
| **High-Risk Commands** | Submit proposal, change parent unit, transfer agent | Zero Stale Tolerance | Synchronous authoritative check inside database transaction |

---

## 3. Account Disablement and Suspension Handling

### Scenario A: Identity Disabled in Keycloak
- When an administrator disables an account in Keycloak, any attempt to refresh the session token via `POST /protocol/openid-connect/token` returns `HTTP 400 Bad Request (invalid_grant: account_disabled)`.
- The BFF session refresh fails, the session is purged from PostgreSQL, and the user is redirected to the login screen.

### Scenario B: Agent Suspended or Terminated
- If an agent is suspended in `iaop-agent-service`:
  1. `get_today` dashboard detects `SUSPENDED` status, marks eligibility as `is_eligible: false`, and renders the `CRITICAL` restriction banner.
  2. Direct selling commands on downstream domain services fail with `HTTP 409 Conflict / 403 Forbidden` (`agent lifecycle status is SUSPENDED — operation prohibited`).
