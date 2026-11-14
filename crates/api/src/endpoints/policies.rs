//! Policy servicing endpoints (Group 6.12: POLICIES-01 .. POLICIES-02)
//!
//! Authoritative integration: downstream Policies/Core Insurance Service required.
//! These endpoints are EXPLICITLY UNAVAILABLE until `POLICIES_SERVICE_URL` is configured.

use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::envelope::{CanonicalErrorCode, FeErrorResponse, FeResponse};
use crate::principal::AuthenticatedPrincipal;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PolicySummaryDto {
    pub policy_id: Uuid,
    pub policy_number: String,
    pub product_name: String,
    pub policyholder_name: String,
    pub sum_assured: String,
    pub status: String, // "IN_FORCE", "LAPSED", "GRACE_PERIOD", "TERMINATED"
    pub next_premium_due_date: Option<DateTime<Utc>>,
    pub issued_at: DateTime<Utc>,
    pub source_freshness_as_of: DateTime<Utc>,
}

fn policies_unavailable(principal: &AuthenticatedPrincipal) -> FeErrorResponse {
    FeErrorResponse::new(
        CanonicalErrorCode::DependencyUnavailable,
        "Policies service is not configured. Downstream iaop-policies-service required.",
        principal.request_id,
        principal.correlation_id,
    )
}

/// GET /api/v1/policies — POLICIES-01
pub async fn list_policies(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<PolicySummaryDto>>, FeErrorResponse> {
    if state.config.policies_service_url.is_none() {
        return Err(policies_unavailable(&principal));
    }
    Err(policies_unavailable(&principal))
}

/// GET /api/v1/policies/{policyId} — POLICIES-02
pub async fn get_policy(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(_policy_id): Path<Uuid>,
) -> Result<FeResponse<PolicySummaryDto>, FeErrorResponse> {
    if state.config.policies_service_url.is_none() {
        return Err(policies_unavailable(&principal));
    }
    Err(policies_unavailable(&principal))
}
