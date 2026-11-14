# IAOP BFF Route-to-Domain Inventory & Status Matrix
**Exhaustive Catalog of Endpoints, Domain Ownership, and Integration Status**

---

| BFF Endpoint | User Journey / Purpose | Downstream Domain Service | Downstream Endpoint | Transformation / Behavior | Status |
|---|---|---|---|---|---|
| `GET /api/v1/me` | Current authenticated agent profile | `iaop-agent-service` | `GET /api/v1/agents/{id}` | Enriches profile with dynamic permissions and active workspace | **PASS** |
| `GET /api/v1/me/workspaces` | Available organizational workspaces | `iaop-agency-service` | `GET /api/v1/agents/{id}/current-assignment`, `GET /api/v1/organization-units/{id}` | Resolves real unit and ancestor workspaces from agency assignments | **PASS** |
| `POST /api/v1/me/workspaces/select` | Switch active workspace context | `iaop-agency-service` | Authoritative permitted workspace check | Rejects unauthorized workspaces with 403; updates session atomically | **PASS** |
| `GET /api/v1/me/today` | Agent dashboard aggregation | `iaop-agent-service`, `iaop-agency-service`, `iaop-eligibility-service` | Multiple downstream queries | Concurrent Tokio join with bounded timeouts; partial section reporting | **PASS** |
| `GET /api/v1/agency/hierarchy` | Organization chart and agency branch | `iaop-agency-service` | `GET /api/v1/organization-units/{id}`, `GET /api/v1/organization-units/{id}/ancestors` | Maps agency units and parent chain to frontend DTO | **PASS** |
| `GET /api/v1/agency/my-team` | Subordinate team member list | `iaop-agency-service` | `GET /api/v1/organization-units/{id}/assignments` | Queries all assigned agents in current unit; eliminates hardcoded empty list | **PASS** |
| `GET /api/v1/recruitment/candidates` | List recruited candidate leads | `iaop-recruitment-service` | N/A (service not deployed) | Explicit HTTP 503 `DependencyUnavailable` (no fake data) | **BLOCKED** |
| `POST /api/v1/recruitment/candidates` | Create candidate application | `iaop-recruitment-service` | N/A (service not deployed) | Explicit HTTP 503 `DependencyUnavailable` (no fake UUID) | **BLOCKED** |
| `GET /api/v1/customers` | Customer portfolio search | `iaop-customer-service` | N/A (service not deployed) | Explicit HTTP 503 `DependencyUnavailable` (no fake list) | **BLOCKED** |
| `POST /api/v1/customers` | Register customer lead | `iaop-customer-service` | N/A (service not deployed) | Explicit HTTP 503 `DependencyUnavailable` (no fake mutation) | **BLOCKED** |
| `GET /api/v1/policies` | Active insurance policies | `iaop-policies-service` | N/A (service not deployed) | Explicit HTTP 503 `DependencyUnavailable` (no fake policy) | **BLOCKED** |
| `GET /api/v1/claims` | Assistance claims tracking | `iaop-claims-service` | N/A (service not deployed) | Explicit HTTP 503 `DependencyUnavailable` (no fake claim) | **BLOCKED** |
| `POST /api/v1/claims/assistance` | Submit claim intake assistance | `iaop-claims-service` | N/A (service not deployed) | Explicit HTTP 503 `DependencyUnavailable` (no fake submission) | **BLOCKED** |
| `GET /health/live` | Liveness probe | None | Local probe | HTTP 200 OK | **PASS** |
| `GET /health/ready` | Readiness probe | PostgreSQL | Pool ping probe | Probes active database connection | **PASS** |
| `GET /api/v1/system/info` | Service build metadata | None | Build info constants | Exposes Git SHA and version | **PASS** |
