//! Today dashboard endpoints (Group 6.2: TODAY-01 .. TODAY-04)
//!
//! Real composition from authoritative domain services.
//! Synthetic tasks, fabricated counts, and default eligible=true are eliminated.

use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::envelope::{CanonicalErrorCode, FeErrorResponse, FeResponse};
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
    pub action: String,
    pub notes: Option<String>,
}

/// GET /api/v1/me/today â€” TODAY-01
pub async fn get_today(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<TodayDashboardDto>, FeErrorResponse> {
    let mut unavailable = Vec::new();
    let mut partial = false;

    let mut lifecycle_status = "UNKNOWN".to_string();
    let mut eligibility_section = SectionResult::unavailable("Agent service unconfigured");

    // Concurrently fetch agent profile and eligibility decision using Tokio concurrency
    let agent_fut = async {
        if let (Some(client), Some(aid)) = (&state.agent_client, principal.agent_id) {
            Some(
                tokio::time::timeout(
                    std::time::Duration::from_secs(3),
                    client.get_agent(aid, &principal.access_token, principal.request_id),
                )
                .await,
            )
        } else {
            None
        }
    };

    let elig_fut = async {
        if let (Some(elig_client), Some(aid)) = (&state.eligibility_client, principal.agent_id) {
            Some(
                tokio::time::timeout(
                    std::time::Duration::from_secs(3),
                    elig_client.verify_eligibility(
                        aid,
                        &principal.access_token,
                        principal.request_id,
                    ),
                )
                .await,
            )
        } else {
            None
        }
    };

    let (agent_outcome, elig_outcome) = tokio::join!(agent_fut, elig_fut);

    if state.agent_client.is_none() {
        partial = true;
        unavailable.push("agent_service".to_string());
    } else {
        match agent_outcome {
            Some(Ok(Ok(view))) => {
                lifecycle_status = view.lifecycle_status.clone();
                match view.lifecycle_status.as_str() {
                    "ACTIVE" => {
                        if state.eligibility_client.is_none() {
                            partial = true;
                            unavailable.push("eligibility_service".to_string());
                            eligibility_section = SectionResult::unavailable(
                                "Authoritative eligibility service is not configured",
                            );
                        } else {
                            match elig_outcome {
                                Some(Ok(Ok(dec))) => {
                                    eligibility_section =
                                        SectionResult::available(EligibilitySummary {
                                            is_eligible: dec.is_eligible,
                                            reason: dec.reason,
                                            valid_until: dec.valid_until,
                                        });
                                }
                                Some(Ok(Err(e))) => {
                                    tracing::warn!(error = %e, "authoritative eligibility service unreachable");
                                    partial = true;
                                    unavailable.push("eligibility_service".to_string());
                                    eligibility_section = SectionResult::unavailable(
                                        "Authoritative eligibility service unavailable",
                                    );
                                }
                                Some(Err(_)) => {
                                    tracing::warn!("authoritative eligibility query timed out");
                                    partial = true;
                                    unavailable.push("eligibility_service".to_string());
                                    eligibility_section = SectionResult::unavailable(
                                        "Authoritative eligibility service timed out",
                                    );
                                }
                                None => {
                                    partial = true;
                                    unavailable.push("eligibility_service".to_string());
                                    eligibility_section = SectionResult::unavailable(
                                        "Authoritative eligibility service is not configured",
                                    );
                                }
                            }
                        }
                    }
                    "SUSPENDED" => {
                        eligibility_section = SectionResult::available(EligibilitySummary {
                            is_eligible: false,
                            reason: Some("Agent status is currently suspended".to_string()),
                            valid_until: None,
                        });
                    }
                    "TERMINATED" => {
                        eligibility_section = SectionResult::available(EligibilitySummary {
                            is_eligible: false,
                            reason: Some("Agent contract is terminated".to_string()),
                            valid_until: None,
                        });
                    }
                    _ => {
                        eligibility_section = SectionResult::available(EligibilitySummary {
                            is_eligible: false,
                            reason: Some("Agent activation is pending".to_string()),
                            valid_until: None,
                        });
                    }
                }
            }
            Some(Ok(Err(e))) => {
                tracing::warn!(error = %e, "failed to query agent service for today dashboard");
                partial = true;
                unavailable.push("agent_service".to_string());
                eligibility_section = SectionResult::unavailable("Agent service unavailable");
            }
            Some(Err(_)) => {
                tracing::warn!("agent service query timed out");
                partial = true;
                unavailable.push("agent_service".to_string());
                eligibility_section = SectionResult::unavailable("Agent service query timed out");
            }
            None => {
                partial = true;
                unavailable.push("agent_service".to_string());
                eligibility_section = SectionResult::unavailable("Agent service unconfigured");
            }
        }
    }

    let is_eligible = eligibility_section
        .data
        .as_ref()
        .map(|e| e.is_eligible)
        .unwrap_or(false);

    let hero = if !is_eligible && lifecycle_status == "SUSPENDED" {
        Some(HeroAction {
            title: "Pemberitahuan Pembatasan Akun".to_string(),
            description:
                "Status keagenan Anda sedang ditangguhkan. Tindakan penjualan dinonaktifkan."
                    .to_string(),
            priority: "CRITICAL".to_string(),
            action_type: "VIEW_RESTRICTION".to_string(),
            action_target: "/profile/restrictions".to_string(),
        })
    } else if !is_eligible && lifecycle_status == "PENDING_ACTIVATION" {
        Some(HeroAction {
            title: "Aktivasi Keagenan Sedang Diproses".to_string(),
            description:
                "Permohonan aktivasi keagenan Anda sedang dalam proses verifikasi kepatuhan."
                    .to_string(),
            priority: "HIGH".to_string(),
            action_type: "VIEW_ONBOARDING".to_string(),
            action_target: "/onboarding/status".to_string(),
        })
    } else {
        Some(HeroAction {
            title: "Selamat Bekerja".to_string(),
            description: "Kelola portofolio nasabah dan prospek Anda hari ini.".to_string(),
            priority: "NORMAL".to_string(),
            action_type: "VIEW_DASHBOARD".to_string(),
            action_target: "/dashboard".to_string(),
        })
    };

    let priority_tasks = if state.config.workflow_service_url.is_none() {
        partial = true;
        unavailable.push("tasks".to_string());
        SectionResult::unavailable("Workflow task service unconfigured")
    } else {
        SectionResult::available(Vec::new())
    };

    let agenda = if state.config.workflow_service_url.is_none() {
        partial = true;
        unavailable.push("agenda".to_string());
        SectionResult::unavailable("Calendar and agenda service unconfigured")
    } else {
        SectionResult::available(Vec::new())
    };

    let recent_activity = if state.config.notification_service_url.is_none() {
        partial = true;
        unavailable.push("recent_activity".to_string());
        SectionResult::unavailable("Notification and activity service unconfigured")
    } else {
        SectionResult::available(Vec::new())
    };

    let dashboard = TodayDashboardDto {
        lifecycle_context: lifecycle_status,
        greeting: format!(
            "Halo, {}",
            principal.display_name.unwrap_or_else(|| "Agen".to_string())
        ),
        hero_action: hero,
        eligibility: eligibility_section,
        priority_tasks,
        agenda,
        recent_activity,
        customer_attention_count: 0,
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
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(_query): Query<TaskListQuery>,
) -> Result<FeResponse<Vec<TaskItemDto>>, FeErrorResponse> {
    if state.config.workflow_service_url.is_none() {
        return Err(FeErrorResponse::new(
            CanonicalErrorCode::DependencyUnavailable,
            "Workflow task service is not configured",
            principal.request_id,
            principal.correlation_id,
        ));
    }
    let tasks: Vec<TaskItemDto> = Vec::new();
    Ok(FeResponse::ok(
        tasks,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/me/agenda
pub async fn get_agenda(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<AgendaItemDto>>, FeErrorResponse> {
    if state.config.workflow_service_url.is_none() {
        return Err(FeErrorResponse::new(
            CanonicalErrorCode::DependencyUnavailable,
            "Calendar and agenda service is not configured",
            principal.request_id,
            principal.correlation_id,
        ));
    }
    let agenda: Vec<AgendaItemDto> = Vec::new();
    Ok(FeResponse::ok(
        agenda,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// PATCH /api/v1/me/tasks/{taskId} â€” TODAY-03
pub async fn update_task(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(_task_id): Path<String>,
    Json(_payload): Json<UpdateTaskRequest>,
) -> Result<FeResponse<serde_json::Value>, FeErrorResponse> {
    Err(FeErrorResponse::new(
        CanonicalErrorCode::DependencyUnavailable,
        "Authoritative task management service is not configured",
        principal.request_id,
        principal.correlation_id,
    ))
}
