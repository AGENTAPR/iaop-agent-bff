# AGENA — Agent BFF Post-Implementation Audit (Partner Journey)

Date: 2026-11-15 · Environment: InstaBox DEV (`iaop-keycloak-dev`)
Scope: Agent Experience / Partner Journey only. Operations BFF and
Office/Internal journeys explicitly out of scope.

## A. Previous implementation summary (verified, not report-trusted)

- BFF `agency_client` snake_case DTOs + ancestors-as-IDs fix (PR #17).
- BFF `agent_client` snake_case DTOs + AppointmentView types (PR #18).
- Agency `current-assignment` single-object contract (PR #16).
- Identity resolution: BFF login → Agent Service `identity-links/resolve`;
  Agency claim-less fallback → same endpoint (agency PR #17).
- Lifecycle endpoints agency+unit (PRs #18/#19 area), error mapping
  (PR #23), idempotency fingerprint hash (PR #21), unsafe-mode guards
  (agent#17, agency#25).
- Portal dedicated catalog pages (contracts PR #18); agency catalog
  `e2e_verified` @ `a57afbb` (contracts PR #17).
- Event schema parity (event-contracts PR #7).

## B. Repository SHAs (origin/development)

| Repo | SHA |
|---|---|
| iaop-agent-service | `691f7b32` |
| iaop-agency-service | `419c607f` |
| iaop-agent-bff | `fe001934` |
| iaop-api-contracts | `329fc1dd` |
| iaop-event-contracts | `3e2b3631` |
| iaop-operations-bff | `2d43b141` (inspected — no domain REST clients) |

## C. Consumer dependency matrix (actual call sites)

### Agency client (`agency_client.rs`)

| Method | Upstream endpoint | BFF call sites | Status |
|---|---|---|---|
| get_current_assignment | GET /agents/{id}/current-assignment | agency.rs:70,177; identity.rs:182 | CONSUMED (workspaces, hierarchy, my-team) |
| get_unit | GET /organization-units/{id} | agency.rs:107,125; identity.rs:187,215 | CONSUMED (ancestor+unit resolution) |
| get_ancestors | GET /organization-units/{id}/ancestors | agency.rs:98; identity.rs:206 | CONSUMED |
| get_unit_assignments | GET /organization-units/{id}/assignments | agency.rs:193 | CONSUMED (my-team) |
| list_agencies | GET /agencies | — | NOT CONSUMED by Agent BFF |

### Agent client (`agent_client.rs`)

| Method | Upstream endpoint | BFF call sites | Status |
|---|---|---|---|
| resolve_identity | GET /identity-links/resolve | auth_routes.rs:237 | CONSUMED (login) |
| get_agent | GET /agents/{id} | agency.rs:217, identity.rs:274, sales.rs:146, today.rs:150 | CONSUMED |
| get_agent_by_code | GET /agents/code/{code} | — | NOT CONSUMED |
| get_appointments | GET /agents/{id}/appointments | — | NOT CONSUMED |
| get_identity_links | GET /agents/{id}/identity-links | — | NOT CONSUMED |

### Operations BFF compatibility

`iaop-operations-bff` contains no HTTP clients to Agent/Agency services
(only messaging/security/persistence/config crates). The
`current-assignment` array→object change and snake_case contracts
cannot break it. No Operations changes required or made.

## D. Partner Journey coverage (live DEV evidence)

| # | Scenario | Result | Evidence |
|---|---|---|---|
| 1 | Agent login (OIDC code+PKCE) | PASS | 303 callback, `iaop_session` cookie |
| 2 | Authoritative identity resolution | PASS | `agentId` populated at callback; claim-less Agency token resolves via identity-links |
| 3 | Profile + lifecycle status | PASS | `/api/v1/me` → `AGT-QA-0001`, `ACTIVE` |
| 4 | Current assignment | PASS | workspace resolution → Unit R PRIMARY |
| 5 | Agency + hierarchy | PASS | `/agency/hierarchy` → Unit R node, ancestors resolved |
| 6 | My Team | PASS | fixture: AGT-QA-0002 assigned to Unit R → member listed (see F-1) |
| 7 | Downline visibility | PARTIAL | my-team is same-unit scoped; org-wide downline = documented gap (F-1) |
| 8 | Cross-agency/cross-unit denial | PASS | live: 2 out-of-scope units → 403, own unit → 200 |
| 9 | Inactive/revoked identity | CODE-VERIFIED | `LINK_REVOKED` path restricts to candidate mode; live unlink test not executed |
| 10 | Domain error translation | PASS | 401 no/forged cookie; BFF envelope + canonical error codes |

## E. Agent Service integration status

- `resolve_identity`, `get_agent` consumed; both snake_case — fixed (PR #18).
- Agency→Agent validation: service-account client credentials live
  (assignment creation 201 verified agent ACTIVE).
- Agent lifecycle commands exist; activation requires appointment +
  eligibility decision — DEV `ACTIVATION_MODE` unset → `fail_closed`
  (activation returns INDETERMINATE by design).

## F. Findings

- **F-1 (documented gap, not a defect)**: Agent Service
  `authorize_agent_read` allows self-or-staff reads only; org-wide
  manager/downline scoping "requires agency membership data and remains
  a documented gap — non-staff callers are denied rather than assumed"
  (`agents.rs` ~line 74). Consequence: BFF my-team enrichment returns
  `agentCode`/`lifecycleStatus` = null for other agents (verified live:
  member listed with null enrichment; upstream 403 in agent-service
  logs). BFF degrades explicitly — no fabricated values. Resolving
  leader-reads-downline requires a business/design decision on where
  org-scope authorization lives → REQUIREMENT_UNRESOLVED.
- **F-2**: DEV fixture agent AGT-QA-0002 created via API then activated
  via direct DB update (activation path blocked by fail_closed
  eligibility) — QA fixture only, lifecycle history row recorded.

## G. Database verification

Isolated PostgreSQL 16 (disposable containers, this session):
- agent-service db_tests 2/2 PASSED; agency-service db_tests 10/10
  PASSED (concurrent reparent, authz negatives, idempotency, temporal
  hierarchy, outbox fencing/poison).

## H. Kafka verification

- Live outbox delivery confirmed (agency.agent-assigned.v1 retry→publish;
  agent.created published to `iaop.agent.created` topic observed in logs).
- Event schema parity: event-contracts PR #7 aligned all emitted
  payloads; envelope pattern accepts `.v1` names.
- Live broker-kill failure injection: NOT_EXECUTED.

## I. Scalar/OpenAPI parity

- Agency: runtime 17 paths/20 ops == canonical spec (parity verified);
  catalog `e2e_verified` @ deployed `a57afbb`.
- Agent: canonical spec present; catalog `implemented_verified` /
  `e2e_verified`.
- Portal serves updated catalog; `/dev-api/agency` proxy verified
  (session+Bearer → 200; missing either → 401).

## J. Security test results

- Missing/forged session → 401 (BFF); no-auth domain → 401.
- Cross-unit resource denial → 403 (live).
- Unsafe modes (dev auth, fixture authz) → local-only, unit-tested.
- Domain error mapping fixed: 409/412/422 verified live (no 500s).

## K. Out-of-scope changes

None. Operations BFF untouched; no new endpoints; no migrations added
this pass; shared domain admin features preserved.

## L. Remaining defects / open items

- F-1 leader downline read scope: REQUIREMENT_UNRESOLVED (business
  decision needed on org-scope authorization ownership).
- Live Kafka failure-injection, perf benchmarks: NOT_EXECUTED.
- Agent activation E2E blocked by fail_closed eligibility in DEV
  (documented dependency, not a defect).

## M. Verdict

`CONDITIONALLY DEV READY` — every Partner Journey scenario has live DEV
evidence except leader-downline enrichment depth (documented domain
gap) and inactive-identity live test; no unresolved defects in the
BFF↔Domain contract layer.
