# IAOP Security & Agent BFF — Final Remediation Report

Date: 2026-11-15 · Branch: `feature/bff-security-p0-remediation` (iaop-agent-bff), `feature/agent-resource-authz` (iaop-agent-service)

## A. Repository results

| Repository | Starting SHA | Final SHA | Changes | Tests | Status |
|---|---|---|---|---|---|
| iaop-agent-bff | 81a88d66 | 522ac5d (PR #12) | Verified JWT permissions, fail-closed workspace authz, atomic session mutations, synthetic-data removal, fail-closed token encryption, request-path token refresh w/ rotation+singleflight, readiness docs corrected | NOT EXECUTED — local MSVC C++ toolchain broken (`ring`/`vcruntime.h`); needs CI or dev machine | PARTIAL |
| iaop-agent-service | 551b211b | branch HEAD | Resource-level read authorization (self-or-staff) on all agent read endpoints | NOT EXECUTED — same toolchain blocker | PARTIAL |
| iaop-agency-service | 266c8da1 | unchanged | Org-scope resource authz NOT implemented — requires subject→agent_id resolution client (see blockers) | — | BLOCKED |
| iaop-rust-common | 7402c86b | 620bd63 (PR #8 → development) | JWKS singleflight refresh, stale-key grace window, concurrency test | NOT EXECUTED — same toolchain blocker | PARTIAL |

## B. Defect closure

| Audit ID | Original defect | Implementation | Evidence | Result |
|---|---|---|---|---|
| §3 P0 | Workspace fail-open when agency client unconfigured | `resolve_workspace_resolution()` returns Member/Candidate; unconfigured/unreachable Agency Service → hard reject for linked agents; candidate workspace only for legitimately unlinked principals | code: `endpoints/identity.rs` | FIXED (unverified compile) |
| §4 P0 | `extract_permissions_from_jwt` decoded unverified JWT, merged all resource_access roles | Replaced by `verified_permissions()` — JWKS-validated access token via new `access_validator` (aud `iaop-agent-api`); empty grants on failure | code: `identity.rs`, `auth_routes.rs`, `main.rs` | FIXED (unverified compile) |
| §5 P0 | `let _ = update_session` ignored persistence errors; lost-update race | `persist_workspace()` atomic single-column UPDATE with rows_affected check; `update_session` reduced to activity touch | code: `session.rs`, `pg_session.rs`, `identity.rs` | FIXED (unverified compile) |
| §6 P0 (agent) | `agent:read` allowed reading ANY agent | `authorize_agent_read`: staff roles OR ACTIVE identity link matching target agent_id; list endpoint staff-only | code: `agents.rs` | FIXED (unverified compile) |
| §6 P0 (agency) | No org-scope resource authz | NOT DONE — no subject→agent resolution path exists in agency-service | — | BLOCKED |
| §7 P0 | Synthetic team/dashboard data | `TeamMemberDto`/`AgencyHierarchyNodeDto`/dashboard counts now `Option` — `None` until authoritative source exists; concurrent JoinSet enrichment (no N+1, no fabricated names/codes/counts) | code: `agency.rs`, `today.rs` | FIXED (unverified compile) |
| §9 P1 (partial) | Token refresh never invoked in request path | `ensure_fresh_session` in principal extractor: 60s skew refresh, per-session singleflight, atomic encrypted `update_tokens`, rotation via COALESCE, revoked→session invalidated, transient→503 | code: `principal.rs`, `oidc.rs`, `session.rs`, `pg_session.rs` | FIXED (unverified compile) |
| §11 P1 (partial) | JWKS cache missing singleflight/stale policy | Singleflight refresh lock + double-check, stale-key grace (3x TTL), concurrency test | code: `jwks.rs` (rust-common PR #8) | FIXED (unverified compile) |
| §12 P1 (partial) | Plaintext token fallback when DB configured without key | `SESSIONS_ENCRYPTION_KEY` mandatory whenever `DATABASE_URL` set; `PgSessionStore` fails closed without cipher | code: `config.rs`, `pg_session.rs` | FIXED (unverified compile) |
| §2/§20 | "CERTIFIED PRODUCTION READY" claim inaccurate | Correction notices added to 5 readiness docs; final report verdict NOT READY | docs commits | DONE |
| §8, §10, §13–§15 P1 | Identity error semantics completion, dynamic revocation, performance fundamentals, Kafka/outbox re-verification, reference-standard contract tests | Not addressed | — | NOT DONE |

## C. Security matrix (implemented, pending runtime verification)

| Scenario | Expected | Implementation |
|---|---|---|
| Arbitrary workspace select | 403 | permitted-list check via Agency Service |
| Agency Service down during select | 503 fail-closed | `DependencyUnavailable` |
| Unlinked user workspace select | candidate workspace only | `WorkspaceResolution::Candidate` |
| Invalid-signature token → permissions | empty | JWKS validation failure → `Vec::new()` |
| Self-service read other agent | 403 | `authorize_agent_read` identity-link check |
| Staff read any agent | 200 | staff role bypass |
| Non-staff list agents | 403 | `is_staff_reader` gate |

## D. BFF route matrix

Not regenerated this session — see `IAOP_BFF_ROUTE_TO_DOMAIN_MATRIX.md` (stale; requires re-audit after merge).

## E. Demo-user tests

NOT EXECUTED — blocked on local toolchain (see G). Demo users (`demo.agent.1-3`, read-only `agent:read`/`agent:history:read`) remain the intended negative-test subjects: they must now receive 403 on other agents' records.

## F. Performance results

NOT MEASURED — no fabricated benchmarks published.

## G. Remaining blockers

1. **Local Rust toolchain broken**: `ring v0.17.14` fails (`vcruntime.h` missing; `vcvars64.bat` errors). All `cargo check/test/clippy` gates BLOCKED until CI or a machine with VS C++ workload runs them. PRs #12 (bff) and #13 (agent-service) are OPEN and MERGEABLE but unverified by compiler.
2. **Agency-service org-scope authz**: needs subject→agent_id resolution (agent-service `resolve_identity` client — `AGENT_SERVICE_URL` config exists but is unused) plus unit-hierarchy scope checks. Not implemented.
3. **Token exchange (RFC 8693)**: not attempted; multi-audience forwarding remains the compatibility mode.
4. **JWKS hardening, session encryption audit, revocation freshness, k6 benchmarks**: not started.
5. **Branch protection**: now ACTIVE via rulesets (all 31 repos public); `development` requires PR + approval — merges need owner approval.

## Open PRs awaiting owner approval (ruleset-enforced)

- iaop-agent-bff PR #12 → development (5 commits)
- iaop-agent-service PR #13 → development
- iaop-rust-common PR #8 → development

## H. Final verdict

**NOT READY** — P0 defects in BFF and agent-service have code-level fixes on open PRs, but nothing is compile-verified, runtime-tested, or merged. Agency-service resource authz, all P1 items, real E2E, and performance gates remain open. The previous "CERTIFIED PRODUCTION READY" claim in `PRODUCTION_READINESS_REPORT.md` is inaccurate and must not be relied upon.
