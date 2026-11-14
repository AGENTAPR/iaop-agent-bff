# IAOP Domain Service Security Standard
**Authoritative Security Contract for All Present and Future IAOP Microservices**

---

## 1. Scope & Objective
This document defines the mandatory security architecture, authorization patterns, and middleware interfaces required for every IAOP domain microservice (e.g. Agent Service, Agency Service, Eligibility Service, Commission Service).

---

## 2. Invariant Security Principles

### Principle 1: Zero Trust of Ingress Tiers
Domain services must never assume callers (including the Agent BFF or Integration Gateway) have performed valid authorization. Every incoming request must present an `Authorization: Bearer <JWT>` header and be validated independently.

### Principle 2: Strict Algorithm and Key Management
1. Only `RS256` (RSA Signature with SHA-256) is accepted. Symmetrical algorithms (`HS256`, `HS384`, `HS512`) and `none` algorithm are strictly prohibited and rejected before key parsing.
2. Keys must be retrieved over TLS from the IdP JWKS endpoint (`/.well-known/jwks.json` or `/protocol/openid-connect/certs`).
3. JWKS caching must implement:
   - Positive TTL (e.g. 5 minutes).
   - Rate-limited refresh on unknown `kid` (minimum 30 seconds between requests) to prevent DoS attacks against the IdP.

### Principle 3: Audience Isolation
Every service must define a dedicated audience identifier (e.g., `OIDC_AUDIENCE=iaop-agent-api`). The validator must reject any token whose `aud` claim does not contain the service identifier, preventing cross-service token reuse.

---

## 3. Reference Implementation: `iaop-security` Crate

All domain services must depend on the shared `iaop-security` library:

```rust
use iaop_security::jwks::{JwtValidator, RemoteJwksProvider};

pub struct JwksTokenValidator {
    inner: JwtValidator<RemoteJwksProvider>,
}

impl JwksTokenValidator {
    pub fn new(issuer: String, jwks_url: String, audience: String) -> Self {
        Self {
            inner: JwtValidator::new(RemoteJwksProvider::new(jwks_url), issuer, audience),
        }
    }
}
```

### Context & Principal Mapping
```rust
impl TokenValidator for JwksTokenValidator {
    async fn validate(&self, token: &str) -> Result<Principal, AppError> {
        let claims = self.inner.validate::<TokenClaims>(token).await
            .map_err(|e| AppError::Forbidden(format!("unauthenticated: {e}")))?;

        Ok(Principal {
            subject: claims.sub,
            issuer: claims.iss,
            scopes: claims.scope.unwrap_or_default().split_whitespace().map(str::to_string).collect(),
            roles: extract_audience_scoped_roles(&claims),
            agent_scope: claims.agent_scope,
        })
    }
}
```

---

## 4. Resource-Level ABAC & Organizational Scoping

### Rules for Self vs. Administrative Access:
1. **Agent Self-Access:** When `Principal` possesses role `IAOP_AGENT` or scope `agent:read` without administrative privileges, target resource ID (`agent_id`) must match the principal's linked agent identity (`agent_scope`).
2. **Manager Hierarchy Scoping:** Supervisors and Agency Unit Leaders may only inspect subordinate units and assigned agents. Cross-agency access without platform-wide administrative roles (`IAOP_PLATFORM_ADMIN`, `AGENT_ADMIN`) results in HTTP 403 Forbidden.
3. **Fail-Closed Validation:** Missing external validators (e.g., `AGENT_SERVICE_URL` unset in Agency Service) must instantiate `FailClosedAgentValidator` which denies all validation requests.
