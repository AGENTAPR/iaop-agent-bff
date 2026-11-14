//! Typed application configuration loaded from environment variables.

use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    MissingVar(&'static str),
    #[error("invalid value for {0}: {1}")]
    InvalidValue(&'static str, String),
    #[error("unsafe configuration: {0}")]
    UnsafeConfig(String),
}

/// Service configuration. Secrets (e.g. DATABASE_URL) are never logged;
/// use [`AppConfig::redacted_summary`] for safe diagnostics.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub service_name: String,
    pub service_version: String,
    pub environment: String,
    pub bind_addr: String,
    /// Optional: services without persistence leave this unset and
    /// `/health/ready` does not probe a database.
    pub database_url: Option<String>,
    pub database_max_connections: u32,
    pub request_timeout: Duration,
    /// OIDC/BFF auth — all optional; auth routes return 503 when unset.
    pub oidc_issuer: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    pub oidc_redirect_uri: Option<String>,
    pub app_base_url: String,
    pub session_idle_timeout_secs: u64,
    pub session_absolute_timeout_secs: u64,
    pub sessions_encryption_key: Option<String>,
    pub oidc_allowed_return_paths: Vec<String>,
    pub cors_allowed_origins: Vec<String>,
    pub agent_service_url: Option<String>,
    pub agency_service_url: Option<String>,
    pub recruitment_service_url: Option<String>,
    pub onboarding_service_url: Option<String>,
    pub learning_service_url: Option<String>,
    pub certification_service_url: Option<String>,
    pub sales_service_url: Option<String>,
    pub eligibility_service_url: Option<String>,
    pub portfolio_service_url: Option<String>,
    pub commission_service_url: Option<String>,
    pub compliance_service_url: Option<String>,
    pub workflow_service_url: Option<String>,
    pub notification_service_url: Option<String>,
    pub document_service_url: Option<String>,
    pub performance_service_url: Option<String>,
    pub claims_service_url: Option<String>,
    pub customer_service_url: Option<String>,
    pub policies_service_url: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let return_paths = std::env::var("OIDC_ALLOWED_RETURN_PATHS")
            .unwrap_or_else(|_| "/,/dashboard".into())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let cors_origins = std::env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let environment = env_or("ENVIRONMENT", "local");
        let is_production = environment.eq_ignore_ascii_case("production");
        let database_url = std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty());
        let sessions_encryption_key = std::env::var("SESSIONS_ENCRYPTION_KEY")
            .or_else(|_| std::env::var("BFF_SESSION_KEY"))
            .ok()
            .filter(|v| !v.is_empty());

        // Persistent session storage (PostgreSQL) must never fall back to
        // plaintext OAuth tokens — encryption key is mandatory whenever a
        // database is configured, in every environment.
        if database_url.is_some() && sessions_encryption_key.is_none() {
            return Err(ConfigError::UnsafeConfig(
                "SESSIONS_ENCRYPTION_KEY must be configured when DATABASE_URL is set (persistent sessions require encrypted token storage)".into(),
            ));
        }

        if is_production && database_url.is_none() {
            return Err(ConfigError::UnsafeConfig(
                "DATABASE_URL must be configured when ENVIRONMENT=production".into(),
            ));
        }

        Ok(Self {
            service_name: env_or("SERVICE_NAME", "iaop-agent-bff"),
            service_version: env_or("SERVICE_VERSION", env!("CARGO_PKG_VERSION")),
            environment,
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:8080"),
            database_url,
            database_max_connections: env_parse("DATABASE_MAX_CONNECTIONS", 10)?,
            request_timeout: Duration::from_secs(env_parse("REQUEST_TIMEOUT_SECS", 30)?),
            oidc_issuer: std::env::var("OIDC_ISSUER").ok().filter(|v| !v.is_empty()),
            oidc_client_id: std::env::var("OIDC_CLIENT_ID")
                .ok()
                .filter(|v| !v.is_empty()),
            oidc_client_secret: std::env::var("OIDC_CLIENT_SECRET")
                .ok()
                .filter(|v| !v.is_empty()),
            oidc_redirect_uri: std::env::var("OIDC_REDIRECT_URI")
                .ok()
                .filter(|v| !v.is_empty()),
            app_base_url: env_or("APP_BASE_URL", "http://localhost:8080"),
            session_idle_timeout_secs: env_parse("SESSION_IDLE_TIMEOUT_SECS", 3600)?,
            session_absolute_timeout_secs: env_parse("SESSION_ABSOLUTE_TIMEOUT_SECS", 43200)?,
            sessions_encryption_key,
            oidc_allowed_return_paths: return_paths,
            cors_allowed_origins: cors_origins,
            agent_service_url: std::env::var("AGENT_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            agency_service_url: std::env::var("AGENCY_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            recruitment_service_url: std::env::var("RECRUITMENT_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            onboarding_service_url: std::env::var("ONBOARDING_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            learning_service_url: std::env::var("LEARNING_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            certification_service_url: std::env::var("CERTIFICATION_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            sales_service_url: std::env::var("SALES_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            eligibility_service_url: std::env::var("ELIGIBILITY_SERVICE_URL")
                .or_else(|_| std::env::var("COMPLIANCE_SERVICE_URL"))
                .ok()
                .filter(|v| !v.is_empty()),
            portfolio_service_url: std::env::var("PORTFOLIO_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            commission_service_url: std::env::var("COMMISSION_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            compliance_service_url: std::env::var("COMPLIANCE_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            workflow_service_url: std::env::var("WORKFLOW_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            notification_service_url: std::env::var("NOTIFICATION_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            document_service_url: std::env::var("DOCUMENT_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            performance_service_url: std::env::var("PERFORMANCE_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            claims_service_url: std::env::var("CLAIMS_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            customer_service_url: std::env::var("CUSTOMER_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            policies_service_url: std::env::var("POLICIES_SERVICE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
        })
    }

    /// Diagnostics-safe summary: never includes secrets.
    pub fn redacted_summary(&self) -> String {
        format!(
            "service={} version={} env={} bind={} db={} timeout={}s",
            self.service_name,
            self.service_version,
            self.environment,
            self.bind_addr,
            if self.database_url.is_some() {
                "configured"
            } else {
                "disabled"
            },
            self.request_timeout.as_secs(),
        )
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_parse<T: std::str::FromStr>(key: &'static str, default: T) -> Result<T, ConfigError> {
    match std::env::var(key) {
        Ok(v) => v
            .parse::<T>()
            .map_err(|_| ConfigError::InvalidValue(key, v)),
        Err(_) => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_loads_with_defaults() {
        let cfg = AppConfig::from_env().expect("defaults must be valid");
        assert!(!cfg.service_name.is_empty());
        assert!(cfg.redacted_summary().contains("db="));
    }
}
