use std::sync::Arc;

use crate::auth_routes::AuthContext;
use iaop_agent_bff_infrastructure::config::AppConfig;
use sqlx::PgPool;

/// Shared application state injected into handlers via `State`.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    /// `None` when DATABASE_URL is not configured; `/health/ready`
    /// reports accordingly instead of failing.
    pub db: Option<PgPool>,
    /// OIDC/BFF auth context; `None` disables /auth/* (503).
    pub auth: Option<std::sync::Arc<AuthContext>>,
    /// Client for the authoritative Agent Service.
    pub agent_client: Option<std::sync::Arc<iaop_agent_bff_infrastructure::clients::AgentClient>>,
    /// Client for the authoritative Agency Service.
    pub agency_client: Option<std::sync::Arc<iaop_agent_bff_infrastructure::clients::AgencyClient>>,
    /// Client for authoritative selling eligibility decisions.
    pub eligibility_client:
        Option<std::sync::Arc<iaop_agent_bff_infrastructure::clients::EligibilityClient>>,
}
