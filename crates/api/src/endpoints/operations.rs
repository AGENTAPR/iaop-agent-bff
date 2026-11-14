//! Async operation status and personal compliance boundaries (Group 6.16)

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
pub struct OperationStatusDto {
    pub operation_id: Uuid,
    pub status: String, // "ACCEPTED", "RUNNING", "PENDING_CONFIRMATION", "SUCCEEDED", "FAILED", "REQUIRES_REVIEW"
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OffboardingStatusDto {
    pub is_offboarding: bool,
    pub reason: Option<String>,
    pub effective_date: Option<DateTime<Utc>>,
    pub pending_handovers_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentRestrictionDto {
    pub restriction_type: String, // "SELLING_SUSPENDED", "UNDER_AUDIT", "COMMISSION_HOLD"
    pub reason: String,
    pub applied_at: DateTime<Utc>,
    pub action_required: Option<String>,
}

/// GET /api/v1/operations/{operationId}
pub async fn get_operation_status(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(operation_id): Path<Uuid>,
) -> Result<FeResponse<OperationStatusDto>, FeErrorResponse> {
    let op = OperationStatusDto {
        operation_id,
        status: "SUCCEEDED".to_string(),
        description: "Operasi bisnis berhasil diproses".to_string(),
        created_at: Utc::now() - chrono::Duration::seconds(45),
        completed_at: Some(Utc::now()),
        error_details: None,
    };

    Ok(FeResponse::ok(
        op,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/me/offboarding-status — OFFBOARDING-04
pub async fn get_my_offboarding_status(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<OffboardingStatusDto>, FeErrorResponse> {
    let status = OffboardingStatusDto {
        is_offboarding: false,
        reason: None,
        effective_date: None,
        pending_handovers_count: 0,
    };

    Ok(FeResponse::ok(
        status,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/me/restrictions — COMPLIANCE-01 (Agent read view)
pub async fn get_my_restrictions(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<AgentRestrictionDto>>, FeErrorResponse> {
    // Return empty list if no active restrictions
    let restrictions = Vec::new();
    Ok(FeResponse::ok(
        restrictions,
        principal.request_id,
        principal.correlation_id,
    ))
}
