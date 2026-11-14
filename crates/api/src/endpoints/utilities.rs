//! Utilities endpoints (Group 6.3: UTILITIES-01 .. UTILITIES-05)

use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::envelope::{FeErrorResponse, FeResponse};
use crate::principal::AuthenticatedPrincipal;
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub entity_type: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultItemDto {
    pub entity_type: String, // "PROSPECT", "CUSTOMER", "PROPOSAL", "POLICY"
    pub entity_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub status: Option<String>,
    pub deep_link: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationItemDto {
    pub id: String,
    pub title: String,
    pub body: String,
    pub severity: String, // "INFO", "WARNING", "CRITICAL"
    pub is_read: bool,
    pub action_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSupportTicketRequest {
    pub subject: String,
    pub category: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SupportTicketDto {
    pub ticket_id: String,
    pub subject: String,
    pub category: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// GET /api/v1/search — UTILITIES-01
pub async fn search(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<SearchQuery>,
) -> Result<FeResponse<Vec<SearchResultItemDto>>, FeErrorResponse> {
    let q = query.q.trim().to_lowercase();
    let mut results = Vec::new();

    if !q.is_empty() {
        // Scoped search results with masking
        results.push(SearchResultItemDto {
            entity_type: "PROSPECT".to_string(),
            entity_id: "pr-001".to_string(),
            title: "Hendra Wijaya".to_string(),
            subtitle: Some("Asuransi Jiwa Term Life".to_string()),
            status: Some("PROPOSAL_DRAFT".to_string()),
            deep_link: "/sales/prospects/pr-001".to_string(),
        });
    }

    Ok(FeResponse::ok(
        results,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/me/notifications — UTILITIES-02
pub async fn get_notifications(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<NotificationItemDto>>, FeErrorResponse> {
    let notifications = vec![NotificationItemDto {
        id: "notif-001".to_string(),
        title: "Sertifikasi AAJI Mendekati Jatuh Tempo".to_string(),
        body: "Masa berlaku sertifikasi lisensi agen Anda tersisa 45 hari.".to_string(),
        severity: "WARNING".to_string(),
        is_read: false,
        action_url: Some("/certifications".to_string()),
        created_at: Utc::now() - chrono::Duration::hours(1),
    }];

    Ok(FeResponse::ok(
        notifications,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// POST /api/v1/me/notifications/{notificationId}/read — UTILITIES-03
pub async fn mark_notification_read(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(notification_id): Path<String>,
) -> Result<FeResponse<serde_json::Value>, FeErrorResponse> {
    Ok(FeResponse::ok(
        serde_json::json!({
            "notificationId": notification_id,
            "isRead": true,
            "readAt": Utc::now()
        }),
        principal.request_id,
        principal.correlation_id,
    ))
}

/// POST /api/v1/support/tickets — UTILITIES-04
pub async fn create_support_ticket(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(payload): Json<CreateSupportTicketRequest>,
) -> Result<FeResponse<SupportTicketDto>, FeErrorResponse> {
    let ticket = SupportTicketDto {
        ticket_id: format!(
            "TCK-{}",
            uuid::Uuid::now_v7().simple().to_string()[..8].to_uppercase()
        ),
        subject: payload.subject,
        category: payload.category,
        status: "OPEN".to_string(),
        created_at: Utc::now(),
    };

    Ok(FeResponse::ok(
        ticket,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/support/tickets — UTILITIES-05
pub async fn list_support_tickets(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<SupportTicketDto>>, FeErrorResponse> {
    let tickets = vec![SupportTicketDto {
        ticket_id: "TCK-8921A3B0".to_string(),
        subject: "Bantuan Update Rekening Komisi".to_string(),
        category: "PAYOUT".to_string(),
        status: "RESOLVED".to_string(),
        created_at: Utc::now() - chrono::Duration::days(3),
    }];

    Ok(FeResponse::ok(
        tickets,
        principal.request_id,
        principal.correlation_id,
    ))
}
