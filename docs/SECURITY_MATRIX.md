# AGENA IAOP Agent BFF — Security Matrix

This matrix details the security controls, authentication mechanisms, CSRF protection, tenant and resource isolation, field-level masking, and negative test verification implemented across the service.

## 1. Threat Mitigations & Controls

| Area | Control | Implementation Mechanism | Verified By |
|---|---|---|---|
| **Authentication** | Backend-managed OIDC Authorization Code + PKCE | Authorization code exchanged server-side; tokens stored in server session; client never receives access or refresh tokens | `crates/infrastructure/src/oidc.rs`, `crates/api/src/auth_routes.rs` |
| **Token Theft** | HttpOnly + SameSite Cookie | Session ID stored in `__Host-iaop_session` cookie (`HttpOnly; Secure; SameSite=Lax; Path=/`) | `auth_routes.rs:set_cookie` |
| **Login CSRF** | State & Browser Binding Cookie | `LoginTransaction.browser_binding` verified against `__Host-iaop_login_binding` cookie | `auth_routes.rs:callback` |
| **State-Changing CSRF** | Double-Submit Token | State-mutating routes (`POST`, `PUT`, `PATCH`, `DELETE`) require `x-csrf-token` header matching session `csrf_token` | `principal.rs:from_request_parts`, `mutating_request_without_csrf_returns_403` |
| **Session Fixation** | UUID v7 ID Generation | New opaque session ID issued upon successful authentication callback; old transaction destroyed | `auth_routes.rs:callback` |
| **Replay Attacks** | Atomic Transaction Consumption | Single-use login transaction consumed via `take_login_txn` (DELETE RETURNING in Postgres) | `session.rs:txn_single_use`, `pg_session.rs:take_login_txn` |
| **Multi-Tenancy** | Session-Scoped Workspace Context | Workspace context validated against principal authorization; tenant ID cannot be overridden via request body | `principal.rs:AuthenticatedPrincipal`, `identity.rs:select_workspace` |
| **PII & Masking** | Masked Customer National ID (KTP) | NIK/KTP masked to show only first 4 digits followed by asterisks (`3171************`) | `customers.rs:list_customers` |
| **Direct Document Relay** | Pre-signed Upload/Download URLs | Raw document bytes are NEVER accepted or relayed via BFF JSON | `documents.rs:create_upload_intent` |
| **Privilege Escalation** | Operations BFF Separation | Administrative endpoints (integration retries, agent suspension, wide audit) excluded from Agent BFF | Route segregation in `routes.rs` |
| **Monetary Precision** | Exact Decimal Strings | All financial figures represented as formatted string decimals; floating-point math forbidden | `sales.rs:ProductDto`, `earnings.rs:EarningsSummaryDto` |

## 2. Endpoint Authorization & CSRF Matrix

| Endpoint Route | HTTP Method | Authentication | CSRF Header Required | Resource Ownership | Negative Test Result |
|---|---|---|---|---|---|
| `/health/live` | GET | None (Public) | No | None | PASS |
| `/health/ready` | GET | None (Public) | No | None | PASS |
| `/api/v1/system/info` | GET | None (Public) | No | None | PASS |
| `/auth/login` | GET | None | No | Browser Session | PASS |
| `/auth/callback` | GET | None | No | Single-use Transaction | PASS (State replay rejected) |
| `/auth/session` | GET | Session Cookie | No | Calling Principal | PASS |
| `/auth/logout` | POST | Session Cookie | Yes (`x-csrf-token`) | Active Session | PASS (403 on missing CSRF) |
| `/api/v1/me` | GET | Session Cookie | No | Calling Principal | PASS (401 on missing session) |
| `/api/v1/me/workspaces` | GET | Session Cookie | No | Calling Principal | PASS (401 on missing session) |
| `/api/v1/me/workspaces/select` | POST | Session Cookie | Yes (`x-csrf-token`) | Calling Principal | PASS (403 on missing CSRF) |
| `/api/v1/me/preferences` | GET | Session Cookie | No | Calling Principal | PASS (401 on missing session) |
| `/api/v1/me/preferences` | PATCH | Session Cookie | Yes (`x-csrf-token`) | Calling Principal | PASS (403 on missing CSRF) |
| `/api/v1/me/today` | GET | Session Cookie | No | Calling Principal | PASS (401 on missing session) |
| `/api/v1/me/tasks` | GET | Session Cookie | No | Assigned Agent | PASS (401 on missing session) |
| `/api/v1/me/tasks/{id}` | PATCH | Session Cookie | Yes (`x-csrf-token`) | Assigned Agent | PASS (403 on missing CSRF) |
| `/api/v1/search` | GET | Session Cookie | No | Calling Principal Scope | PASS (401 on missing session) |
| `/api/v1/me/notifications` | GET | Session Cookie | No | Calling Principal | PASS (401 on missing session) |
| `/api/v1/sales/products` | GET | Session Cookie | No | Product Catalog | PASS |
| `/api/v1/sales/illustrations`| POST | Session Cookie | Yes (`x-csrf-token`) | Calling Principal | PASS (403 on missing CSRF) |
| `/api/v1/sales/proposals` | POST | Session Cookie | Yes (`x-csrf-token`) | Calling Principal + Active Status | PASS (403 when suspended) |
| `/api/v1/documents/upload-intent` | POST | Session Cookie | Yes (`x-csrf-token`) | Calling Principal | PASS (422 if size > 20MB) |
