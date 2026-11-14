//! Claim assistance endpoints (Group 6.13: CLAIMS-01 .. CLAIMS-03)
//!
//! Architectural rule: Agent BFF owns claim assistance intake and tracking only;
//! adjudication authority belongs exclusively to Core Claims service.

use axum::Json;
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

/// GET /api/v1/claims — CLAIMS-01
pub async fn list_claims(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<ClaimAssistanceDto>>, FeErrorResponse> {
    let claims = vec![ClaimAssistanceDto {
        case_id: Uuid::now_v7(),
        case_number: "CLM-2024-00192".to_string(),
        policy_id: Uuid::now_v7(),
        claimant_name: "Hendra Wijaya".to_string(),
        incident_date: Utc::now() - chrono::Duration::days(10),
        status: "DOCS_REVIEW".to_string(),
        claim_type: "HOSPITALIZATION".to_string(),
        estimated_amount: "15000000.00".to_string(),
        created_at: Utc::now() - chrono::Duration::days(5),
    }];

    Ok(FeResponse::ok(
        claims,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// POST /api/v1/claims/assistance — CLAIMS-02
pub async fn create_claim_assistance(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(payload): Json<CreateClaimAssistanceRequest>,
) -> Result<FeResponse<ClaimAssistanceDto>, FeErrorResponse> {
    let case_id = Uuid::now_v7();
    let num = format!("CLM-{}", case_id.simple().to_string()[..8].to_uppercase());

    let case = ClaimAssistanceDto {
        case_id,
        case_number: num,
        policy_id: payload.policy_id,
        claimant_name: payload.claimant_name,
        incident_date: payload.incident_date,
        status: "SUBMITTED".to_string(),
        claim_type: payload.claim_type,
        estimated_amount: payload.estimated_amount,
        created_at: Utc::now(),
    };

    Ok(FeResponse::ok(
        case,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/claims/{claimId} — CLAIMS-03
pub async fn get_claim(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(claim_id): Path<Uuid>,
) -> Result<FeResponse<ClaimAssistanceDto>, FeErrorResponse> {
    let case = ClaimAssistanceDto {
        case_id: claim_id,
        case_number: "CLM-2024-00192".to_string(),
        policy_id: Uuid::now_v7(),
        claimant_name: "Hendra Wijaya".to_string(),
        incident_date: Utc::now() - chrono::Duration::days(10),
        status: "DOCS_REVIEW".to_string(),
        claim_type: "HOSPITALIZATION".to_string(),
        estimated_amount: "15000000.00".to_string(),
        created_at: Utc::now() - chrono::Duration::days(5),
    };

    Ok(FeResponse::ok(
        case,
        principal.request_id,
        principal.correlation_id,
    ))
}
