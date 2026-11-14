//! Document management endpoints (Group 6.6: DOCUMENTS-01 .. DOCUMENTS-03)
//!
//! Architectural rule: Never relay raw document bytes through BFF JSON.
//! Upload and download utilize secure pre-signed URLs provided by Document Service.

use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::envelope::{FeErrorResponse, FeResponse};
use crate::principal::AuthenticatedPrincipal;
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadIntentRequest {
    pub file_name: String,
    pub mime_type: String,
    pub document_type: String, // "KTP", "AAJI_LICENSE", "PROPOSAL_SIGNATURE", "CLAIM_PROOF"
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadIntentDto {
    pub document_id: Uuid,
    pub upload_url: String,
    pub expires_at: DateTime<Utc>,
    pub required_headers: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DownloadIntentDto {
    pub document_id: Uuid,
    pub download_url: String,
    pub expires_at: DateTime<Utc>,
}

/// POST /api/v1/documents/upload-intent — DOCUMENTS-01
pub async fn create_upload_intent(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(payload): Json<UploadIntentRequest>,
) -> Result<FeResponse<UploadIntentDto>, FeErrorResponse> {
    if payload.size_bytes > 20 * 1024 * 1024 {
        return Err(FeErrorResponse::validation(
            "File size exceeds maximum allowed limit (20MB)",
            vec![],
            principal.request_id,
            principal.correlation_id,
        ));
    }

    let doc_id = Uuid::now_v7();
    let intent = UploadIntentDto {
        document_id: doc_id,
        upload_url: format!("https://storage.iaop.internal/uploads/{}?sig=valid", doc_id),
        expires_at: Utc::now() + chrono::Duration::minutes(15),
        required_headers: serde_json::json!({
            "Content-Type": payload.mime_type
        }),
    };

    Ok(FeResponse::ok(
        intent,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// POST /api/v1/documents/{documentId}/complete — DOCUMENTS-02
pub async fn complete_upload(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(document_id): Path<Uuid>,
) -> Result<FeResponse<serde_json::Value>, FeErrorResponse> {
    Ok(FeResponse::ok(
        serde_json::json!({
            "documentId": document_id,
            "status": "PROCESSING",
            "virusScan": "PENDING",
            "completedAt": Utc::now()
        }),
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/documents/{documentId}/download-intent — DOCUMENTS-03
pub async fn create_download_intent(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(document_id): Path<Uuid>,
) -> Result<FeResponse<DownloadIntentDto>, FeErrorResponse> {
    let intent = DownloadIntentDto {
        document_id,
        download_url: format!(
            "https://storage.iaop.internal/downloads/{}?sig=valid",
            document_id
        ),
        expires_at: Utc::now() + chrono::Duration::minutes(10),
    };

    Ok(FeResponse::ok(
        intent,
        principal.request_id,
        principal.correlation_id,
    ))
}
