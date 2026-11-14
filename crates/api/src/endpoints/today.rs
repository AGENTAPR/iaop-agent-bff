//! Today dashboard endpoints (Group 6.2: TODAY-01 .. TODAY-04)

use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::envelope::{FeErrorResponse, FeResponse};
use crate::principal::AuthenticatedPrincipal;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SectionResult<T> {
    pub status: String, // "AVAILABLE", "STALE", "UNAVAILABLE"
    pub as_of: DateTime<Utc>,
    pub data: Option<T>,
    pub error_reason: Option<String>,
}

impl<T> SectionResult<T> {
    pub fn available(data: T) -> Self {
        Self {
            status: "AVAILABLE".to_string(),
            as_of: Utc::now(),
            data: Some(data),
            error_reason: None,
        }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            status: "UNAVAILABLE".to_string(),
            as_of: Utc::now(),
            data: None,
            error_reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HeroAction {
    pub title: String,
    pub description: String,
    pub priority: String, // "CRITICAL", "HIGH", "NORMAL"
    pub action_type: String,
    pub action_target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EligibilitySummary {
    pub is_eligible: bool,
    pub reason: Option<String>,
    pub valid_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskItemDto {
    pub id: String,
    pub origin_service: String,
    pub business_entity: String,
    pub task_type: String,
    pub title: String,
    pub priority: String,
    pub due_date: Option<DateTime<Utc>>,
    pub status: String,
    pub allowed_actions: Vec<String>,
    pub deep_link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgendaItemDto {
    pub id: String,
    pub title: String,
    pub origin_service: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status: String,
    pub entity_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActivityItemDto {
    pub id: String,
    pub activity_type: String,
    pub description: String,
    pub occurred_at: DateTime<Utc>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TodayDashboardDto {
    pub lifecycle_context: String,
    pub greeting: String,
    pub hero_action: Option<HeroAction>,
    pub eligibility: SectionResult<EligibilitySummary>,
    pub priority_tasks: SectionResult<Vec<TaskItemDto>>,
    pub agenda: SectionResult<Vec<AgendaItemDto>>,
    pub recent_activity: SectionResult<Vec<ActivityItemDto>>,
    pub customer_attention_count: u32,
    pub open_claims_count: u32,
    pub as_of: DateTime<Utc>,
    pub partial: bool,
    pub unavailable_sections: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TaskListQuery {
    pub status: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskRequest {
    pub action: String, // e.g. "START", "SUBMIT", "CANCEL"
    pub notes: Option<String>,
}

/// GET /api/v1/me/today — TODAY-01
pub async fn get_today(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<TodayDashboardDto>, FeErrorResponse> {
    let mut unavailable = Vec::new();
    let mut partial = false;

    let mut lifecycle_status = "ACTIVE".to_string();
    let mut eligibility_eligible = true;
    let mut eligibility_reason = None;

    // Check agent service for authoritative status
    if let (Some(client), Some(aid)) = (&state.agent_client, principal.agent_id) {
        match client
            .get_agent(aid, &principal.access_token, principal.request_id)
            .await
        {
            Ok(view) => {
                lifecycle_status = view.lifecycle_status.clone();
                if view.lifecycle_status == "SUSPENDED" {
                    eligibility_eligible = false;
                    eligibility_reason = Some("Agent status is currently suspended".to_string());
                } else if view.lifecycle_status == "TERMINATED" {
                    eligibility_eligible = false;
                    eligibility_reason = Some("Agent contract terminated".to_string());
                }
            }
            Err(_) => {
                partial = true;
                unavailable.push("agent_service".to_string());
            }
        }
    }

    // Server priority logic:
    // 1. Critical restriction/blocker
    // 2. Urgent action
    // 3. Customer follow-up
    // 4. Default greeting
    let hero = if !eligibility_eligible {
        Some(HeroAction {
            title: "Pemberitahuan Pembatasan Akun".to_string(),
            description: eligibility_reason
                .clone()
                .unwrap_or_else(|| "Status keagenan tidak memenuhi syarat penjualan".to_string()),
            priority: "CRITICAL".to_string(),
            action_type: "VIEW_RESTRICTION".to_string(),
            action_target: "/profile/restrictions".to_string(),
        })
    } else {
        Some(HeroAction {
            title: "Tindak Lanjuti Calon Nasabah".to_string(),
            description: "2 prospek baru menunggu penawaran ilustrasi hari ini".to_string(),
            priority: "NORMAL".to_string(),
            action_type: "NAVIGATE_SALES".to_string(),
            action_target: "/sales/prospects".to_string(),
        })
    };

    let eligibility = SectionResult::available(EligibilitySummary {
        is_eligible: eligibility_eligible,
        reason: eligibility_reason,
        valid_until: Some(Utc::now() + chrono::Duration::days(30)),
    });

    let tasks = SectionResult::available(vec![TaskItemDto {
        id: "tsk-001".to_string(),
        origin_service: "sales-service".to_string(),
        business_entity: "prospect".to_string(),
        task_type: "FOLLOW_UP".to_string(),
        title: "Kirim ilustrasi proteksi ke Bpk. Hendra".to_string(),
        priority: "HIGH".to_string(),
        due_date: Some(Utc::now() + chrono::Duration::hours(4)),
        status: "OPEN".to_string(),
        allowed_actions: vec!["COMPLETE".to_string(), "RESCHEDULE".to_string()],
        deep_link: Some("/sales/proposals/new?prospectId=pr-001".to_string()),
    }]);

    let agenda = SectionResult::available(vec![AgendaItemDto {
        id: "agn-001".to_string(),
        title: "Pertemuan Evaluasi Bulanan Agensi".to_string(),
        origin_service: "agency-service".to_string(),
        start_time: Utc::now() + chrono::Duration::hours(2),
        end_time: Utc::now() + chrono::Duration::hours(3),
        status: "SCHEDULED".to_string(),
        entity_reference: Some("agency-meeting".to_string()),
    }]);

    let recent_activity = SectionResult::available(vec![ActivityItemDto {
        id: "act-001".to_string(),
        activity_type: "LOGIN_SUCCESS".to_string(),
        description: "Sesi kerja baru berhasil dibuka".to_string(),
        occurred_at: Utc::now() - chrono::Duration::minutes(5),
        correlation_id: principal.correlation_id.map(|c| c.to_string()),
    }]);

    let name = principal.display_name.as_deref().unwrap_or("Agen");
    let greeting = format!("Selamat datang, {}!", name);

    let dashboard = TodayDashboardDto {
        lifecycle_context: lifecycle_status,
        greeting,
        hero_action: hero,
        eligibility,
        priority_tasks: tasks,
        agenda,
        recent_activity,
        customer_attention_count: 2,
        open_claims_count: 0,
        as_of: Utc::now(),
        partial,
        unavailable_sections: unavailable,
    };

    Ok(FeResponse::ok(
        dashboard,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/me/tasks — TODAY-02
pub async fn get_tasks(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(_query): Query<TaskListQuery>,
) -> Result<FeResponse<Vec<TaskItemDto>>, FeErrorResponse> {
    let tasks = vec![TaskItemDto {
        id: "tsk-001".to_string(),
        origin_service: "sales-service".to_string(),
        business_entity: "prospect".to_string(),
        task_type: "FOLLOW_UP".to_string(),
        title: "Kirim ilustrasi proteksi ke Bpk. Hendra".to_string(),
        priority: "HIGH".to_string(),
        due_date: Some(Utc::now() + chrono::Duration::hours(4)),
        status: "OPEN".to_string(),
        allowed_actions: vec!["COMPLETE".to_string(), "RESCHEDULE".to_string()],
        deep_link: Some("/sales/proposals/new?prospectId=pr-001".to_string()),
    }];

    Ok(FeResponse::ok(
        tasks,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// PATCH /api/v1/me/tasks/{taskId} — TODAY-03
pub async fn update_task(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(task_id): Path<String>,
    Json(payload): Json<UpdateTaskRequest>,
) -> Result<FeResponse<serde_json::Value>, FeErrorResponse> {
    Ok(FeResponse::ok(
        serde_json::json!({
            "taskId": task_id,
            "appliedAction": payload.action,
            "status": "UPDATED",
            "updatedAt": Utc::now()
        }),
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/me/agenda — TODAY-04
pub async fn get_agenda(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<AgendaItemDto>>, FeErrorResponse> {
    let agenda = vec![AgendaItemDto {
        id: "agn-001".to_string(),
        title: "Pertemuan Evaluasi Bulanan Agensi".to_string(),
        origin_service: "agency-service".to_string(),
        start_time: Utc::now() + chrono::Duration::hours(2),
        end_time: Utc::now() + chrono::Duration::hours(3),
        status: "SCHEDULED".to_string(),
        entity_reference: Some("agency-meeting".to_string()),
    }];

    Ok(FeResponse::ok(
        agenda,
        principal.request_id,
        principal.correlation_id,
    ))
}
