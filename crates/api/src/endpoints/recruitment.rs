//! Recruitment endpoints (Group 6.4: RECRUITMENT-01 .. RECRUITMENT-04)
//!
//! Authoritative integration: downstream Recruitment Service required.
//! No candidate data is generated, persisted, or returned without
//! a real `iaop-recruitment-service` backing these calls.

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
pub struct CandidateDto {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub status: String, // "APPLIED", "SCREENING", "INTERVIEW", "OFFERED", "HIRED", "REJECTED"
    pub referrer_agent_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCandidateRequest {
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReferralSummaryDto {
    pub total_referred: u32,
    pub in_progress: u32,
    pub successfully_onboarded: u32,
    pub recent_candidates: Vec<CandidateDto>,
}

fn recruitment_unavailable(principal: &AuthenticatedPrincipal) -> FeErrorResponse {
    FeErrorResponse::new(
        CanonicalErrorCode::DependencyUnavailable,
        "Recruitment service is not configured. Downstream iaop-recruitment-service required.",
        principal.request_id,
        principal.correlation_id,
    )
}

/// GET /api/v1/recruitment/candidates — RECRUITMENT-01
pub async fn list_candidates(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<CandidateDto>>, FeErrorResponse> {
    if state.config.recruitment_service_url.is_none() {
        return Err(recruitment_unavailable(&principal));
    }
    // Recruitment service integration not yet implemented.
    Err(recruitment_unavailable(&principal))
}

/// POST /api/v1/recruitment/candidates — RECRUITMENT-02
/// Creates a recruitment candidate via the authoritative Recruitment Service.
/// Returns 503 if the service is not configured — never synthesizes a candidate ID.
pub async fn create_candidate(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(payload): Json<CreateCandidateRequest>,
) -> Result<FeResponse<CandidateDto>, FeErrorResponse> {
    // Input validation first (before any downstream call)
    if payload.full_name.trim().is_empty() || payload.email.trim().is_empty() {
        return Err(FeErrorResponse::validation(
            "Name and email are required",
            vec![],
            principal.request_id,
            principal.correlation_id,
        ));
    }

    if state.config.recruitment_service_url.is_none() {
        return Err(recruitment_unavailable(&principal));
    }

    // Recruitment service integration not yet implemented.
    // Will delegate to recruitment-service POST /api/v1/candidates when available.
    Err(recruitment_unavailable(&principal))
}

/// GET /api/v1/recruitment/candidates/{candidateId} — RECRUITMENT-03
pub async fn get_candidate(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(_candidate_id): Path<Uuid>,
) -> Result<FeResponse<CandidateDto>, FeErrorResponse> {
    if state.config.recruitment_service_url.is_none() {
        return Err(recruitment_unavailable(&principal));
    }
    Err(recruitment_unavailable(&principal))
}

/// GET /api/v1/recruitment/my-referrals — RECRUITMENT-04
pub async fn get_my_referrals(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<ReferralSummaryDto>, FeErrorResponse> {
    if state.config.recruitment_service_url.is_none() {
        return Err(recruitment_unavailable(&principal));
    }
    Err(recruitment_unavailable(&principal))
}
