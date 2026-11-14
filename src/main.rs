//! Composition root for Agent Experience BFF.
//!
//! Wires configuration, persistence, and the HTTP router, then serves with
//! graceful shutdown on SIGINT/SIGTERM.

use std::sync::Arc;

use anyhow::Context;
use iaop_agent_bff_api::{AppState, build_router};
use iaop_agent_bff_infrastructure::config::AppConfig;
use iaop_agent_bff_infrastructure::persistence;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env().context("failed to load configuration")?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .json()
        .init();

    tracing::info!(config = %config.redacted_summary(), "starting service");

    let db = match &config.database_url {
        Some(url) => {
            let pool = persistence::connect_pool(url, config.database_max_connections)
                .await
                .context("failed to connect to database")?;
            persistence::run_migrations(&pool)
                .await
                .context("failed to run migrations")?;
            tracing::info!("database migrations applied");
            Some(pool)
        }
        None => {
            tracing::warn!("DATABASE_URL not set; running without persistence");
            None
        }
    };

    let auth = build_auth_context(&config);
    let agent_client = config.agent_service_url.as_ref().map(|url| {
        Arc::new(iaop_agent_bff_infrastructure::clients::AgentClient::new(
            url.clone(),
        ))
    });

    let state = AppState {
        config: Arc::new(config.clone()),
        db,
        auth,
        agent_client,
    };
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .with_context(|| format!("failed to bind {}", config.bind_addr))?;
    tracing::info!(addr = %config.bind_addr, "listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    tracing::info!("shutdown complete");
    Ok(())
}

fn build_auth_context(
    config: &AppConfig,
) -> Option<std::sync::Arc<iaop_agent_bff_api::auth_routes::AuthContext>> {
    use iaop_agent_bff_api::auth_routes::AuthContext;
    use iaop_agent_bff_infrastructure::oidc::OidcConfig;
    use iaop_agent_bff_infrastructure::session::InMemorySessionStore;
    use iaop_security::jwks::{JwtValidator, RemoteJwksProvider};

    let (Some(issuer), Some(client_id), Some(client_secret), Some(redirect_uri)) = (
        config.oidc_issuer.clone(),
        config.oidc_client_id.clone(),
        config.oidc_client_secret.clone(),
        config.oidc_redirect_uri.clone(),
    ) else {
        tracing::warn!("OIDC_* env not fully set; /auth/* disabled");
        return None;
    };
    let oidc = OidcConfig {
        issuer: issuer.clone(),
        client_id: client_id.clone(),
        client_secret,
        redirect_uri,
        allowed_return_paths: config.oidc_allowed_return_paths.clone(),
    };
    let id_validator =
        JwtValidator::new(RemoteJwksProvider::new(oidc.jwks_uri()), issuer, client_id);
    Some(std::sync::Arc::new(AuthContext {
        oidc,
        store: Arc::new(InMemorySessionStore::new()),
        id_validator,
    }))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
