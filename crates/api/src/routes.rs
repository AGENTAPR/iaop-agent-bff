//! HTTP route handlers.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use iaop_agent_bff_application::service_info::GetServiceInfoQuery;
use iaop_agent_bff_infrastructure::persistence;
use utoipa::OpenApi;

use crate::dto::{HealthResponse, SystemInfoResponse};
use crate::endpoints;
use crate::middleware;
use crate::openapi::ApiDoc;
use crate::state::AppState;

/// Builds the service router with all baseline endpoints.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Probes and diagnostics
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .route("/api/v1/system/info", get(system_info))
        .route("/api/docs/openapi.json", get(openapi_json))
        // Backend-managed OIDC Auth
        .route("/auth/login", get(crate::auth_routes::login))
        .route("/auth/callback", get(crate::auth_routes::callback))
        .route("/auth/session", get(crate::auth_routes::session_info))
        .route("/auth/me", get(crate::auth_routes::me))
        .route("/auth/logout", post(crate::auth_routes::logout))
        // Group 6.1: Identity
        .route("/api/v1/me", get(endpoints::identity::get_me))
        .route(
            "/api/v1/me/workspaces",
            get(endpoints::identity::get_workspaces),
        )
        .route(
            "/api/v1/me/workspaces/select",
            post(endpoints::identity::select_workspace),
        )
        .route(
            "/api/v1/me/preferences",
            get(endpoints::identity::get_preferences)
                .patch(endpoints::identity::update_preferences),
        )
        // Group 6.2: Today Dashboard & Tasks
        .route("/api/v1/me/today", get(endpoints::today::get_today))
        .route("/api/v1/me/tasks", get(endpoints::today::get_tasks))
        .route(
            "/api/v1/me/tasks/{taskId}",
            patch(endpoints::today::update_task),
        )
        .route("/api/v1/me/agenda", get(endpoints::today::get_agenda))
        // Group 6.3: Utilities & Notifications
        .route("/api/v1/search", get(endpoints::utilities::search))
        .route(
            "/api/v1/me/notifications",
            get(endpoints::utilities::get_notifications),
        )
        .route(
            "/api/v1/me/notifications/{notificationId}/read",
            post(endpoints::utilities::mark_notification_read),
        )
        .route(
            "/api/v1/support/tickets",
            get(endpoints::utilities::list_support_tickets)
                .post(endpoints::utilities::create_support_ticket),
        )
        // Group 6.4: Recruitment (agent-facing)
        .route(
            "/api/v1/recruitment/candidates",
            get(endpoints::recruitment::list_candidates)
                .post(endpoints::recruitment::create_candidate),
        )
        .route(
            "/api/v1/recruitment/candidates/{candidateId}",
            get(endpoints::recruitment::get_candidate),
        )
        .route(
            "/api/v1/recruitment/my-referrals",
            get(endpoints::recruitment::get_my_referrals),
        )
        // Group 6.5: Onboarding (agent-facing)
        .route(
            "/api/v1/onboarding/me",
            get(endpoints::onboarding::get_my_onboarding),
        )
        .route(
            "/api/v1/onboarding/tasks/{taskId}/submit",
            post(endpoints::onboarding::submit_onboarding_task),
        )
        // Group 6.6: Documents
        .route(
            "/api/v1/documents/upload-intent",
            post(endpoints::documents::create_upload_intent),
        )
        .route(
            "/api/v1/documents/{documentId}/complete",
            post(endpoints::documents::complete_upload),
        )
        .route(
            "/api/v1/documents/{documentId}/download-intent",
            get(endpoints::documents::create_download_intent),
        )
        // Group 6.7: Learning
        .route(
            "/api/v1/learning/courses",
            get(endpoints::learning::list_courses),
        )
        .route(
            "/api/v1/learning/courses/{courseId}",
            get(endpoints::learning::get_course),
        )
        .route(
            "/api/v1/learning/courses/{courseId}/enroll",
            post(endpoints::learning::enroll_course),
        )
        .route(
            "/api/v1/learning/my-progress",
            get(endpoints::learning::get_my_learning_progress),
        )
        // Group 6.8: Certification
        .route(
            "/api/v1/certifications/me",
            get(endpoints::certification::get_my_certifications),
        )
        .route(
            "/api/v1/certifications/evidence",
            post(endpoints::certification::submit_certification_evidence),
        )
        // Group 6.9: Agency
        .route(
            "/api/v1/agency/hierarchy",
            get(endpoints::agency::get_hierarchy),
        )
        .route(
            "/api/v1/agency/my-team",
            get(endpoints::agency::get_my_team),
        )
        // Group 6.10: Customers
        .route(
            "/api/v1/customers",
            get(endpoints::customers::list_customers).post(endpoints::customers::create_customer),
        )
        .route(
            "/api/v1/customers/{customerId}",
            get(endpoints::customers::get_customer),
        )
        // Group 6.11: Sales & Proposals
        .route(
            "/api/v1/sales/products",
            get(endpoints::sales::list_products),
        )
        .route(
            "/api/v1/sales/illustrations",
            post(endpoints::sales::calculate_illustration),
        )
        .route(
            "/api/v1/sales/proposals",
            post(endpoints::sales::submit_proposal),
        )
        .route(
            "/api/v1/sales/proposals/{proposalId}",
            get(endpoints::sales::get_proposal),
        )
        // Group 6.12: Policies
        .route("/api/v1/policies", get(endpoints::policies::list_policies))
        .route(
            "/api/v1/policies/{policyId}",
            get(endpoints::policies::get_policy),
        )
        // Group 6.13: Claims
        .route("/api/v1/claims", get(endpoints::claims::list_claims))
        .route(
            "/api/v1/claims/assistance",
            post(endpoints::claims::create_claim_assistance),
        )
        .route(
            "/api/v1/claims/{claimId}",
            get(endpoints::claims::get_claim),
        )
        // Group 6.14: Earnings
        .route(
            "/api/v1/earnings/summary",
            get(endpoints::earnings::get_earnings_summary),
        )
        .route(
            "/api/v1/earnings/statements",
            get(endpoints::earnings::list_statements),
        )
        // Group 6.15: Performance
        .route(
            "/api/v1/performance/metrics",
            get(endpoints::performance::get_performance_metrics),
        )
        .route(
            "/api/v1/performance/rankings",
            get(endpoints::performance::get_rankings),
        )
        // Group 6.16: Operations & Boundaries
        .route(
            "/api/v1/operations/{operationId}",
            get(endpoints::operations::get_operation_status),
        )
        .route(
            "/api/v1/me/offboarding-status",
            get(endpoints::operations::get_my_offboarding_status),
        )
        .route(
            "/api/v1/me/restrictions",
            get(endpoints::operations::get_my_restrictions),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::request_context,
        ))
        .with_state(state)
}

/// Liveness: process is up. Must NOT depend on external systems.
async fn live() -> impl IntoResponse {
    (StatusCode::OK, Json(HealthResponse { status: "ok" }))
}

/// Readiness: dependencies required to serve traffic.
/// - No DB configured: ready (DB intentionally disabled).
/// - DB configured: ready only if a query succeeds.
async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    match &state.db {
        Some(pool) => {
            if persistence::ping(pool).await {
                (StatusCode::OK, Json(HealthResponse { status: "ready" }))
            } else {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(HealthResponse { status: "degraded" }),
                )
            }
        }
        None => (StatusCode::OK, Json(HealthResponse { status: "ready" })),
    }
}

async fn system_info(State(state): State<AppState>) -> impl IntoResponse {
    let info = GetServiceInfoQuery {
        service_name: state.config.service_name.clone(),
        service_version: state.config.service_version.clone(),
        environment: state.config.environment.clone(),
    }
    .execute();
    Json(SystemInfoResponse {
        service: info.service,
        version: info.version,
        environment: info.environment,
        status: info.status,
    })
}

async fn openapi_json() -> impl IntoResponse {
    Json(ApiDoc::openapi())
}
