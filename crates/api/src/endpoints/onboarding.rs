//! Onboarding endpoints (Group 6.5: ONBOARDING-01 .. ONBOARDING-02)

use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::envelope::{FeErrorResponse, FeResponse};
use crate::principal::AuthenticatedPrincipal;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingStepDto {
    pub step_id: String,
    pub title: String,
    pub status: String, // "PENDING", "IN_REVIEW", "COMPLETED", "REJECTED"
    pub required: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingStatusDto {
    pub overall_status: String, // "NOT_STARTED", "IN_PROGRESS", "SUBMITTED", "APPROVED"
    pub progress_percentage: u32,
    pub steps: Vec<OnboardingStepDto>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubmitOnboardingTaskRequest {
    pub document_id: Option<String>,
    pub input_data: Option<serde_json::Value>,
}

/// GET /api/v1/onboarding/me — ONBOARDING-01
pub async fn get_my_onboarding(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<OnboardingStatusDto>, FeErrorResponse> {
    let steps = vec![
        OnboardingStepDto {
            step_id: "step-kyc".to_string(),
            title: "Verifikasi Identitas & KYC".to_string(),
            status: "COMPLETED".to_string(),
            required: true,
            completed_at: Some(Utc::now() - chrono::Duration::days(1)),
        },
        OnboardingStepDto {
            step_id: "step-contract".to_string(),
            title: "Penandatanganan Perjanjian Keagenan".to_string(),
            status: "COMPLETED".to_string(),
            required: true,
            completed_at: Some(Utc::now() - chrono::Duration::hours(12)),
        },
        OnboardingStepDto {
            step_id: "step-aaji".to_string(),
            title: "Unggah Bukti Lisensi AAJI".to_string(),
            status: "COMPLETED".to_string(),
            required: true,
            completed_at: Some(Utc::now() - chrono::Duration::hours(2)),
        },
    ];

    let status = OnboardingStatusDto {
        overall_status: "APPROVED".to_string(),
        progress_percentage: 100,
        steps,
    };

    Ok(FeResponse::ok(
        status,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// POST /api/v1/onboarding/tasks/{taskId}/submit — ONBOARDING-02
pub async fn submit_onboarding_task(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(task_id): Path<String>,
    Json(_payload): Json<SubmitOnboardingTaskRequest>,
) -> Result<FeResponse<serde_json::Value>, FeErrorResponse> {
    Ok(FeResponse::ok(
        serde_json::json!({
            "taskId": task_id,
            "status": "SUBMITTED",
            "submittedAt": Utc::now()
        }),
        principal.request_id,
        principal.correlation_id,
    ))
}
