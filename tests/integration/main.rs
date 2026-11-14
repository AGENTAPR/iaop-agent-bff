//! Integration tests for IAOP Agent BFF.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use chrono::Utc;
use tower::ServiceExt;
use uuid::Uuid;

use iaop_agent_bff_api::auth_routes::AuthContext;
use iaop_agent_bff_api::{AppState, build_router};
use iaop_agent_bff_infrastructure::config::AppConfig;
use iaop_agent_bff_infrastructure::oidc::OidcConfig;
use iaop_agent_bff_infrastructure::session::{InMemorySessionStore, Session, SessionStore};
use iaop_security::jwks::{JwtValidator, RemoteJwksProvider};

fn test_state() -> AppState {
    AppState {
        config: Arc::new(AppConfig::from_env().expect("test config must load")),
        db: None,
        auth: None,
        agent_client: None,
        agency_client: None,
        eligibility_client: None,
    }
}

async fn test_auth_state() -> (AppState, String, String) {
    let cfg = AppConfig::from_env().expect("test config must load");
    let store = Arc::new(InMemorySessionStore::new());

    let session_id = format!("sess-{}", Uuid::now_v7());
    let csrf_token = "csrf-secret-123456789".to_string();

    store
        .create_session(Session {
            id: session_id.clone(),
            subject: "test-user-sub-001".to_string(),
            display_name: Some("Budi Agen Teladan".to_string()),
            access_token: "mock-access-token".to_string(),
            refresh_token: None,
            access_expires_at: Utc::now() + chrono::Duration::hours(2),
            created_at: Instant::now(),
            last_seen: Instant::now(),
            idle_timeout: Duration::from_secs(3600),
            absolute_timeout: Duration::from_secs(43200),
            csrf_token: csrf_token.clone(),
            workspace_id: Some("ws-jakarta".to_string()),
            agent_id: Some(Uuid::now_v7()),
        })
        .await
        .unwrap();

    let oidc = OidcConfig {
        issuer: "https://auth.iaop.internal/realms/dev".to_string(),
        client_id: "iaop-agent-bff".to_string(),
        client_secret: "secret".to_string(),
        redirect_uri: "http://localhost:8080/auth/callback".to_string(),
        allowed_return_paths: vec!["/".to_string(), "/dashboard".to_string()],
    };
    let id_validator = JwtValidator::new(
        RemoteJwksProvider::new(oidc.jwks_uri()),
        &oidc.issuer,
        &oidc.client_id,
    );

    let auth = Some(Arc::new(AuthContext {
        oidc,
        store,
        id_validator,
    }));

    let state = AppState {
        config: Arc::new(cfg),
        db: None,
        auth,
        agent_client: None,
        agency_client: None,
        eligibility_client: None,
    };

    (state, session_id, csrf_token)
}

async fn response_json(response: Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn health_live_returns_ok() {
    let app = build_router(test_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn system_info_returns_metadata() {
    let app = build_router(test_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/system/info")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn unauthenticated_request_returns_401_fe_envelope() {
    let (state, _, _) = test_auth_state().await;
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let json = response_json(response).await;

    assert_eq!(json["error"]["code"], "UNAUTHENTICATED");
    assert!(json["meta"]["requestId"].is_string());
    assert_eq!(json["meta"]["schemaVersion"], "1.0");
}

#[tokio::test]
async fn authenticated_get_me_returns_200_fe_envelope() {
    let (state, session_id, _) = test_auth_state().await;
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/me")
                .header(header::COOKIE, format!("iaop_session={}", session_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;

    assert_eq!(json["data"]["subject"], "test-user-sub-001");
    assert_eq!(json["data"]["displayName"], "Budi Agen Teladan");
    assert_eq!(json["meta"]["schemaVersion"], "1.0");
}

#[tokio::test]
async fn mutating_request_without_csrf_returns_403() {
    let (state, session_id, _) = test_auth_state().await;
    let app = build_router(state);

    let body = serde_json::json!({
        "workspaceId": "ws-jakarta-selatan"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/me/workspaces/select")
                .header(header::COOKIE, format!("iaop_session={}", session_id))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let json = response_json(response).await;
    assert_eq!(json["error"]["code"], "FORBIDDEN");
}

#[tokio::test]
async fn mutating_request_with_valid_csrf_succeeds() {
    let (state, session_id, csrf_token) = test_auth_state().await;
    let app = build_router(state);

    let body = serde_json::json!({
        "workspaceId": "ws-jakarta-selatan"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/me/workspaces/select")
                .header(header::COOKIE, format!("iaop_session={}", session_id))
                .header("x-csrf-token", csrf_token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["data"]["selectedWorkspaceId"], "ws-jakarta-selatan");
}

#[tokio::test]
async fn today_dashboard_returns_sections_and_hero_action() {
    let (state, session_id, _) = test_auth_state().await;
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/me/today")
                .header(header::COOKIE, format!("iaop_session={}", session_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;

    assert!(json["data"]["greeting"].is_string());
    assert!(json["data"]["heroAction"].is_object());
    assert_eq!(json["data"]["priorityTasks"]["status"], "UNAVAILABLE");
    assert_eq!(json["data"]["agenda"]["status"], "UNAVAILABLE");
    assert_eq!(json["data"]["partial"], true);
    assert_eq!(json["meta"]["schemaVersion"], "1.0");
}

#[tokio::test]
async fn sales_products_unconfigured_returns_503() {
    let (state, session_id, _) = test_auth_state().await;
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/sales/products")
                .header(header::COOKIE, format!("iaop_session={}", session_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let json = response_json(response).await;
    assert_eq!(json["error"]["code"], "DEPENDENCY_UNAVAILABLE");
}

#[tokio::test]
async fn calculate_illustration_unconfigured_returns_503() {
    let (state, session_id, csrf_token) = test_auth_state().await;
    let app = build_router(state);

    let payload = serde_json::json!({
        "productCode": "PRD-TL-01",
        "insuredAge": 35,
        "insuredGender": "MALE",
        "sumAssured": "500000000.00",
        "paymentTermYears": 10
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sales/illustrations")
                .header(header::COOKIE, format!("iaop_session={}", session_id))
                .header("x-csrf-token", csrf_token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let json = response_json(response).await;
    assert_eq!(json["error"]["code"], "DEPENDENCY_UNAVAILABLE");
}
