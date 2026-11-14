# IAOP Identity and Workspace Resolution Specification
**Authoritative Identity Linkage and Dynamic Multi-Tenant Workspace Resolution**

---

## 1. Identity Link Resolution Architecture

In IAOP, an external OpenID Connect identity (`issuer` + `subject`) does not inherently equal an internal `agent_id`. Identity linkages are maintained as authoritative domain entities in `iaop-agent-service`.

```mermaid
sequenceDiagram
    autonumber
    actor User as Agent Browser
    participant BFF as iaop-agent-bff
    participant KC as Keycloak (iaop-dev)
    participant AgentSvc as iaop-agent-service
    participant AgencySvc as iaop-agency-service

    User->>BFF: GET /auth/callback?code=...
    BFF->>KC: Token Exchange (AuthCode + PKCE verifier)
    KC-->>BFF: access_token, id_token (sub = KC_UUID)
    BFF->>AgentSvc: GET /api/v1/identity-links/resolve?issuer=...&subject=KC_UUID
    alt Identity Linked
        AgentSvc-->>BFF: HTTP 200 { agentId: UUID, status: ACTIVE }
        BFF->>AgencySvc: GET /api/v1/agents/{agentId}/current-assignment
        AgencySvc-->>BFF: CurrentAssignmentView { unitId: UUID, role: AGENT }
        BFF->>BFF: Store session with verified agentId and default unit workspace
    else Identity Not Linked (Candidate / Onboarding)
        AgentSvc-->>BFF: HTTP 404 Not Found
        BFF->>BFF: Store session with agentId: None, workspace: workspace-candidate
    end
    BFF-->>User: Set-Cookie: iaop_session_secure (Redirect /dashboard)
```

---

## 2. Dynamic Workspace Derivation Rules

Workspaces are resolved dynamically on each query (`GET /api/v1/me/workspaces`) and validated on selection (`POST /api/v1/me/workspaces/select`):
1. **Primary Assigned Unit:** Derived from `agency_client.get_current_assignment(agent_id)`. Yields workspace with unit name, agency code, and assignment role (`PRIMARY`, `SECONDARY`).
2. **Ancestor Units:** Derived from `agency_client.get_ancestors(unit_id)`. Yields management oversight workspaces for unit leaders.
3. **Candidate Workspace:** Fallback granted to unlinked applicants and onboarding candidates without active agency assignments.
4. **Authoritative Switching Validation:** `select_workspace` checks that the target `workspace_id` exists in the permitted workspace list. Attempting to select an unauthorized workspace is rejected with `HTTP 403 Forbidden` and audited.
