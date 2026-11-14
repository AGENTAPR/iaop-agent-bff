# IAOP Permission and Resource Scope Matrix
**Formal Mapping of Technical Permissions, Keycloak Roles, and ABAC Scopes**

---

## 1. Technical Permission Hierarchy

| Technical Permission | Keycloak Client Role | Keycloak Realm Role | Operation Scope | Description |
|---|---|---|---|---|
| `agent:read` | `iaop-agent-api` | `IAOP_AGENT`, `IAOP_PLATFORM_ADMIN` | Agent Profile | Read agent personal registry details |
| `agent:history:read` | `iaop-agent-api` | `IAOP_PLATFORM_ADMIN`, `AGENT_AUDITOR` | Agent History | Audit log of lifecycle status changes |
| `agent:activate` | `iaop-agent-api` | `AGENT_OPERATIONS`, `IAOP_PLATFORM_ADMIN` | Lifecycle Command | Move agent from PENDING_ACTIVATION to ACTIVE |
| `agent:suspend` | `iaop-agent-api` | `AGENT_LIFECYCLE_OFFICER`, `AGENT_ADMIN` | Lifecycle Command | Suspend active selling privileges |
| `agent:terminate` | `iaop-agent-api` | `AGENT_ADMIN`, `IAOP_PLATFORM_ADMIN` | Lifecycle Command | Irrevocably terminate agent contract |
| `agent:identity:link` | `iaop-agent-api` | `AGENT_ADMIN`, `AGENT_OPERATIONS` | Identity Binding | Authoritatively bind IdP issuer + sub to agentId |
| `agency:read` | `iaop-agency-api` | `IAOP_AGENT`, `IAOP_AGENCY_ADMIN` | Organization Read | Read organization units and hierarchy |
| `agency:assignment:read`| `iaop-agency-api`| `IAOP_AGENT`, `IAOP_AGENCY_ADMIN` | Team Read | Read agent-to-unit assignments |
| `agency:assignment:manage`| `iaop-agency-api`| `IAOP_AGENCY_ADMIN`, `IAOP_PLATFORM_ADMIN`| Assignment Mutation | Assign agent to organization unit |
| `agency:transfer:request`| `iaop-agency-api`| `IAOP_AGENCY_ADMIN` | Transfer Mutation | Request inter-unit transfer |
| `agency:transfer:approve`| `iaop-agency-api`| `IAOP_PLATFORM_ADMIN` | Transfer Command | Approve pending agency transfer |

---

## 2. Resource-Level ABAC Constraints

### 2.1 Agent Resource Scope
- **Rule:** If caller is not in `[PLATFORM_ADMIN, AGENT_ADMIN, AGENT_OPERATIONS, AGENT_AUDITOR]`, the caller's linked `agent_id` must match the target `id` path parameter.
- **Enforcement:** Enforced in `iaop-agent-service/crates/api/src/agents.rs` via `authorize_agent_access`.
- **Result:** Attempting to read another agent's profile returns HTTP 403 Forbidden.

### 2.2 Agency Unit Scope
- **Rule:** Non-administrator users can only query assignments for organization units where they are actively assigned or hold a leadership role (`LeadershipAppointment`).
- **Enforcement:** Enforced in `iaop-agency-service/crates/application/src/queries.rs`.
- **Result:** Attempting to query another unit's team returns HTTP 403 Forbidden.
