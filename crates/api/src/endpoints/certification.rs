//! Certification endpoints (Group 6.8: CERTIFICATION-01 .. CERTIFICATION-02)

use axum::Json;
use axum::extract::State;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::envelope::{FeErrorResponse, FeResponse};
use crate::principal::AuthenticatedPrincipal;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CertificationDto {
    pub id: String,
    pub certification_type: String, // "AAJI", "AASI", "CPD_ANNUAL"
    pub certificate_number: String,
    pub issued_by: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub verification_status: String, // "VERIFIED", "PENDING_VERIFICATION", "EXPIRED"
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubmitEvidenceRequest {
    pub certification_type: String,
    pub certificate_number: String,
    pub document_id: Uuid,
    pub valid_until: DateTime<Utc>,
}

/// GET /api/v1/certifications/me — CERTIFICATION-01
pub async fn get_my_certifications(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<CertificationDto>>, FeErrorResponse> {
    let certs = vec![CertificationDto {
        id: "cert-aaji-001".to_string(),
        certification_type: "AAJI".to_string(),
        certificate_number: "AAJI-2024-88912".to_string(),
        issued_by: "Asosiasi Asuransi Jiwa Indonesia".to_string(),
        issued_at: Utc::now() - chrono::Duration::days(320),
        expires_at: Utc::now() + chrono::Duration::days(45),
        verification_status: "VERIFIED".to_string(),
    }];

    Ok(FeResponse::ok(
        certs,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// POST /api/v1/certifications/evidence — CERTIFICATION-02
pub async fn submit_certification_evidence(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(payload): Json<SubmitEvidenceRequest>,
) -> Result<FeResponse<serde_json::Value>, FeErrorResponse> {
    Ok(FeResponse::ok(
        serde_json::json!({
            "certificationType": payload.certification_type,
            "certificateNumber": payload.certificate_number,
            "documentId": payload.document_id,
            "status": "PENDING_VERIFICATION",
            "submittedAt": Utc::now()
        }),
        principal.request_id,
        principal.correlation_id,
    ))
}
