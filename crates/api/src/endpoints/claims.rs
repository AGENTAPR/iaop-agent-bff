//! Claim assistance endpoints (Group 6.13: CLAIMS-01 .. CLAIMS-03)
//!
//! Authoritative integration: downstream Claims Service required.
//! These endpoints are EXPLICITLY UNAVAILABLE until `iaop-claims-service`
//! is deployed and `CLAIMS_SERVICE_URL` is configured.

use axum::Json;
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
pub struct ClaimAssistanceDto {
    pub case_id: Uuid,
    pub case_number: String,
    pub policy_id: Uuid,
    pub claimant_name: String,
    pub incident_date: DateTime<Utc>,
    pub status: String, // "SUBMITTED", "DOCS_REVIEW", "IN_ASSESSMENT", "APPROVED", "SETTLED"
    pub claim_type: String, // "HOSPITALIZATION", "CRITICAL_ILLNESS", "DEATH"
    pub estimated_amount: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateClaimAssistanceRequest {
    pub policy_id: Uuid,
    pub claimant_name: String,
    pub claim_type: String,
    pub incident_date: DateTime<Utc>,
    pub estimated_amount: String,
    pub document_ids: Vec<Uuid>,
}

fn claims_unavailable(principal: &AuthenticatedPrincipal) -> FeErrorResponse {
    FeErrorResponse::new(
        CanonicalErrorCode::DependencyUnavailable,
        "Claims service is not configured. Downstream iaop-claims-service required.",
        principal.request_id,
        principal.correlation_id,
    )
}

/// GET /api/v1/claims — CLAIMS-01
pub async fn list_claims(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<ClaimAssistanceDto>>, FeErrorResponse> {
    if state.config.claims_service_url.is_none() {
        return Err(claims_unavailable(&principal));
    }
    // Claims service integration not yet implemented — explicit 503 instead of fake data.
    Err(claims_unavailable(&principal))
}

/// POST /api/v1/claims/assistance — CLAIMS-02
pub async fn create_claim_assistance(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(_payload): Json<CreateClaimAssistanceRequest>,
) -> Result<FeResponse<ClaimAssistanceDto>, FeErrorResponse> {
    if state.config.claims_service_url.is_none() {
        return Err(claims_unavailable(&principal));
    }
    Err(claims_unavailable(&principal))
}

/// GET /api/v1/claims/{claimId} — CLAIMS-03
pub async fn get_claim(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(_claim_id): Path<Uuid>,
) -> Result<FeResponse<ClaimAssistanceDto>, FeErrorResponse> {
    if state.config.claims_service_url.is_none() {
        return Err(claims_unavailable(&principal));
    }
    Err(claims_unavailable(&principal))
}
