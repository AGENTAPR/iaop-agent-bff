//! Recruitment endpoints (Group 6.4: RECRUITMENT-01 .. RECRUITMENT-04)

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

/// GET /api/v1/recruitment/candidates — RECRUITMENT-01
pub async fn list_candidates(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<CandidateDto>>, FeErrorResponse> {
    let candidates = vec![CandidateDto {
        id: Uuid::now_v7(),
        full_name: "Budi Santoso".to_string(),
        email: "budi.santoso@example.com".to_string(),
        phone: "+628123456789".to_string(),
        status: "SCREENING".to_string(),
        referrer_agent_id: principal.agent_id,
        created_at: Utc::now() - chrono::Duration::days(2),
    }];

    Ok(FeResponse::ok(
        candidates,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// POST /api/v1/recruitment/candidates — RECRUITMENT-02
pub async fn create_candidate(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(payload): Json<CreateCandidateRequest>,
) -> Result<FeResponse<CandidateDto>, FeErrorResponse> {
    if payload.full_name.trim().is_empty() || payload.email.trim().is_empty() {
        return Err(FeErrorResponse::validation(
            "Name and email are required",
            vec![],
            principal.request_id,
            principal.correlation_id,
        ));
    }

    let candidate = CandidateDto {
        id: Uuid::now_v7(),
        full_name: payload.full_name,
        email: payload.email,
        phone: payload.phone,
        status: "APPLIED".to_string(),
        referrer_agent_id: principal.agent_id,
        created_at: Utc::now(),
    };

    Ok(FeResponse::ok(
        candidate,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/recruitment/candidates/{candidateId} — RECRUITMENT-03
pub async fn get_candidate(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(candidate_id): Path<Uuid>,
) -> Result<FeResponse<CandidateDto>, FeErrorResponse> {
    let candidate = CandidateDto {
        id: candidate_id,
        full_name: "Budi Santoso".to_string(),
        email: "budi.santoso@example.com".to_string(),
        phone: "+628123456789".to_string(),
        status: "SCREENING".to_string(),
        referrer_agent_id: principal.agent_id,
        created_at: Utc::now() - chrono::Duration::days(2),
    };

    Ok(FeResponse::ok(
        candidate,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/recruitment/my-referrals — RECRUITMENT-04
pub async fn get_my_referrals(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<ReferralSummaryDto>, FeErrorResponse> {
    let summary = ReferralSummaryDto {
        total_referred: 3,
        in_progress: 1,
        successfully_onboarded: 2,
        recent_candidates: vec![CandidateDto {
            id: Uuid::now_v7(),
            full_name: "Budi Santoso".to_string(),
            email: "budi.santoso@example.com".to_string(),
            phone: "+628123456789".to_string(),
            status: "SCREENING".to_string(),
            referrer_agent_id: principal.agent_id,
            created_at: Utc::now() - chrono::Duration::days(2),
        }],
    };

    Ok(FeResponse::ok(
        summary,
        principal.request_id,
        principal.correlation_id,
    ))
}
