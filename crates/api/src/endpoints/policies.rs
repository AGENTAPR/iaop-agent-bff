//! Policy servicing endpoints (Group 6.12: POLICIES-01 .. POLICIES-02)

use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::envelope::{FeErrorResponse, FeResponse};
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

/// GET /api/v1/policies — POLICIES-01
pub async fn list_policies(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<PolicySummaryDto>>, FeErrorResponse> {
    let policies = vec![PolicySummaryDto {
        policy_id: Uuid::now_v7(),
        policy_number: "POL-ID-20240901".to_string(),
        product_name: "Nusantara Term Life Proteksi Prima".to_string(),
        policyholder_name: "Hendra Wijaya".to_string(),
        sum_assured: "500000000.00".to_string(),
        status: "IN_FORCE".to_string(),
        next_premium_due_date: Some(Utc::now() + chrono::Duration::days(18)),
        issued_at: Utc::now() - chrono::Duration::days(150),
        source_freshness_as_of: Utc::now(),
    }];

    Ok(FeResponse::ok(
        policies,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/policies/{policyId} — POLICIES-02
pub async fn get_policy(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(policy_id): Path<Uuid>,
) -> Result<FeResponse<PolicySummaryDto>, FeErrorResponse> {
    let policy = PolicySummaryDto {
        policy_id,
        policy_number: "POL-ID-20240901".to_string(),
        product_name: "Nusantara Term Life Proteksi Prima".to_string(),
        policyholder_name: "Hendra Wijaya".to_string(),
        sum_assured: "500000000.00".to_string(),
        status: "IN_FORCE".to_string(),
        next_premium_due_date: Some(Utc::now() + chrono::Duration::days(18)),
        issued_at: Utc::now() - chrono::Duration::days(150),
        source_freshness_as_of: Utc::now(),
    };

    Ok(FeResponse::ok(
        policy,
        principal.request_id,
        principal.correlation_id,
    ))
}
