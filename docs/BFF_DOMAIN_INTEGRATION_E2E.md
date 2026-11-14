# BFF to Domain Service End-to-End Integration Report
**Trace of Real Cross-Service Journeys across Agent BFF, Agent Service, and Agency Service**

---

## 1. Verified Core User Journeys

### Journey 1: OIDC Login & Authoritative Identity Resolution
1. User logs in with `demo.agent.1` via Keycloak PKCE authorization code flow.
2. Callback receives single-use authorization code; claims login transaction atomically in PostgreSQL.
3. BFF queries Agent Service `GET /api/v1/identity-links/resolve?issuer=...&subject=...`.
4. Agent Service returns linked `agentId`.
5. Session is stored in PostgreSQL encrypted with AES-256-GCM.

### Journey 2: Permitted Workspace Listing and Switching
1. `GET /api/v1/me/workspaces`:
   - BFF queries `agency_client.get_current_assignment(agent_id)`.
   - BFF queries `agency_client.get_unit(unit_id)`.
   - BFF queries `agency_client.get_ancestors(unit_id)`.
   - Returns real permitted workspaces.
2. `POST /api/v1/me/workspaces/select`:
   - Checks requested workspace against permitted set.
   - Atomically updates `session.workspace_id`.
   - Rejects unauthorized workspaces with HTTP 403 Forbidden.

### Journey 3: Agency Team Aggregation
1. `GET /api/v1/agency/my-team`:
   - Calls Agency Service `GET /api/v1/organization-units/{id}/assignments`.
   - Eliminates hardcoded empty list (`Vec::new()`).
   - Returns real assigned agents within the user's unit.

### Journey 4: Today Dashboard Aggregation
1. `GET /api/v1/me/today`:
   - Concurrently queries Agent Service and Eligibility Service via `tokio::join!`.
   - Truthfully marks unconfigured domains (tasks, agenda, activity) as `UNAVAILABLE` in `unavailable_sections`.
   - Correctly reports `is_eligible: false` and renders restriction hero action when agent status is `SUSPENDED`.
