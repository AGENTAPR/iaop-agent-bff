# IAOP Shared Security Reference Implementation Guide
**Onboarding Reference for Future Microservices (`iaop-rust-common / iaop-security`)**

---

## 1. Quickstart Guide for New Microservices

To onboard a new IAOP domain service (e.g. `iaop-commission-service`):

### Step 1: Add Dependency in `Cargo.toml`
```toml
[dependencies]
iaop-security = { git = "https://github.com/AGENTAPR/iaop-rust-common", branch = "main" }
```

### Step 2: Wire JWKS Validator in Service Bootstrap
```rust
use iaop_security::jwks::{JwtValidator, RemoteJwksProvider};

pub struct ServiceSecurity {
    validator: JwtValidator<RemoteJwksProvider>,
}

impl ServiceSecurity {
    pub fn new(issuer: String, jwks_url: String, audience: String) -> Self {
        Self {
            validator: JwtValidator::new(RemoteJwksProvider::new(jwks_url), issuer, audience),
        }
    }
}
```

### Step 3: Implement Axum Principal Extractor
```rust
#[derive(Clone)]
pub struct AuthPrincipal(pub Principal);

impl<S> FromRequestParts<S> for AuthPrincipal
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts.headers.get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .filter(|v| v.starts_with("Bearer "));

        let Some(bearer) = auth_header else {
            return Err((StatusCode::UNAUTHORIZED, "Missing Bearer token").into_response());
        };

        let token = &bearer[7..];
        // Validate against JWKS and audience
        ...
    }
}
```

### Step 4: Enforce Resource-Level ABAC in Handlers
```rust
pub async fn get_resource(
    AuthPrincipal(principal): AuthPrincipal,
    Path(resource_id): Path<Uuid>,
) -> Result<Json<ResourceDto>, StatusCode> {
    if !principal.is_admin() && principal.agent_scope != Some(resource_id.to_string()) {
        return Err(StatusCode::FORBIDDEN);
    }
    // Execute domain query
    ...
}
```
