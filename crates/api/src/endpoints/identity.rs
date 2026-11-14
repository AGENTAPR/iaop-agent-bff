//! Identity endpoints (Group 6.1: IDENTITY-01 .. IDENTITY-05)

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
pub struct UserProfileDto {
    pub subject: String,
    pub display_name: Option<String>,
    pub agent_id: Option<Uuid>,
    pub agent_code: Option<String>,
    pub lifecycle_status: Option<String>,
    pub active_workspace: Option<String>,
    pub permissions: Vec<String>,
    pub locale: String,
    pub timezone: String,
    pub session_expires_at: Option<DateTime<Utc>>,
    pub partial: bool,
    pub unavailable_sections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDto {
    pub id: String,
    pub name: String,
    pub agency_code: Option<String>,
    pub role: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkspaceListResponse {
    pub workspaces: Vec<WorkspaceDto>,
    pub active_workspace_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SelectWorkspaceRequest {
    pub workspace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserPreferencesDto {
    pub locale: String,
    pub timezone: String,
    pub theme: String,
    pub notification_prefs: serde_json::Value,
    pub version: i64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePreferencesRequest {
    pub locale: Option<String>,
    pub timezone: Option<String>,
    pub theme: Option<String>,
    pub notification_prefs: Option<serde_json::Value>,
    pub expected_version: Option<i64>,
}

/// Helper to extract dynamic permissions and roles from authenticated JWT payload
fn extract_permissions_from_jwt(token: &str) -> Vec<String> {
    use base64::Engine;
    use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return vec![
            "agent:profile:read".to_string(),
            "agent:dashboard:read".to_string(),
        ];
    }
    let payload_str = parts[1];
    let decoded_bytes = URL_SAFE_NO_PAD
        .decode(payload_str)
        .or_else(|_| URL_SAFE.decode(payload_str));

    let Ok(bytes) = decoded_bytes else {
        return vec![
            "agent:profile:read".to_string(),
            "agent:dashboard:read".to_string(),
        ];
    };

    let Ok(val) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return vec![
            "agent:profile:read".to_string(),
            "agent:dashboard:read".to_string(),
        ];
    };

    let mut permissions = Vec::new();
    // 1. realm_access.roles
    if let Some(roles) = val
        .pointer("/realm_access/roles")
        .and_then(|v| v.as_array())
    {
        for r in roles {
            if let Some(s) = r.as_str() {
                permissions.push(s.to_string());
            }
        }
    }
    // 2. resource_access.*.roles
    if let Some(res) = val.pointer("/resource_access").and_then(|v| v.as_object()) {
        for (_client, client_val) in res {
            if let Some(roles) = client_val.get("roles").and_then(|v| v.as_array()) {
                for r in roles {
                    if let Some(s) = r.as_str() {
                        permissions.push(s.to_string());
                    }
                }
            }
        }
    }
    // 3. scope
    if let Some(scope_str) = val.get("scope").and_then(|v| v.as_str()) {
        for s in scope_str.split_whitespace() {
            permissions.push(format!("scope:{}", s));
        }
    }

    if permissions.is_empty() {
        permissions.push("agent:profile:read".to_string());
        permissions.push("agent:dashboard:read".to_string());
    }

    permissions.sort();
    permissions.dedup();
    permissions
}

/// Helper to authoritatively resolve permitted workspaces for the principal
pub async fn resolve_permitted_workspaces(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
) -> Vec<WorkspaceDto> {
    let mut workspaces = Vec::new();

    if let (Some(agency_client), Some(aid)) = (&state.agency_client, principal.agent_id) {
        match agency_client
            .get_current_assignment(aid, &principal.access_token, principal.request_id)
            .await
        {
            Ok(Some(assignment)) => {
                match agency_client
                    .get_unit(
                        assignment.unit_id,
                        &principal.access_token,
                        principal.request_id,
                    )
                    .await
                {
                    Ok(unit) => {
                        workspaces.push(WorkspaceDto {
                            id: unit.id.to_string(),
                            name: unit.name,
                            agency_code: Some(unit.code),
                            role: assignment.assignment_type.clone(),
                            is_default: true,
                        });

                        if let Ok(ancestors) = agency_client
                            .get_ancestors(
                                assignment.unit_id,
                                &principal.access_token,
                                principal.request_id,
                            )
                            .await
                        {
                            for anc in ancestors {
                                workspaces.push(WorkspaceDto {
                                    id: anc.id.to_string(),
                                    name: anc.name,
                                    agency_code: Some(anc.code),
                                    role: "PARENT_UNIT".to_string(),
                                    is_default: false,
                                });
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, unit_id = %assignment.unit_id, "Failed to resolve agency unit for workspace");
                    }
                }
            }
            Ok(None) => {
                tracing::debug!(agent_id = %aid, "Agent has no active agency assignment; candidate workspace granted");
            }
            Err(e) => {
                tracing::warn!(error = %e, agent_id = %aid, "Failed to query agency assignment");
            }
        }
    }

    if workspaces.is_empty() {
        let default_id = principal
            .workspace_id
            .clone()
            .unwrap_or_else(|| "workspace-candidate".to_string());
        workspaces.push(WorkspaceDto {
            id: default_id,
            name: "Default Agent Workspace".to_string(),
            agency_code: None,
            role: "AGENT".to_string(),
            is_default: true,
        });
    }

    workspaces
}

/// GET /api/v1/me — IDENTITY-01
pub async fn get_me(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<UserProfileDto>, FeErrorResponse> {
    let mut agent_code = None;
    let mut lifecycle_status = None;
    let agent_id = principal.agent_id;
    let mut partial = false;
    let mut unavailable_sections = Vec::new();

    // Query agent details if agent_client is configured
    if let Some(client) = &state.agent_client {
        if let Some(aid) = agent_id {
            match client
                .get_agent(aid, &principal.access_token, principal.request_id)
                .await
            {
                Ok(view) => {
                    agent_code = Some(view.agent_code);
                    lifecycle_status = Some(view.lifecycle_status);
                }
                Err(e) => {
                    tracing::warn!(error = %e, agent_id = %aid, "Failed to fetch agent profile from Agent Service");
                    partial = true;
                    unavailable_sections.push("agent_profile".to_string());
                }
            }
        }
    }

    let permitted = resolve_permitted_workspaces(&state, &principal).await;
    let active_workspace = principal
        .workspace_id
        .clone()
        .filter(|id| permitted.iter().any(|w| &w.id == id))
        .or_else(|| permitted.first().map(|w| w.id.clone()));

    let permissions = extract_permissions_from_jwt(&principal.access_token);

    let profile = UserProfileDto {
        subject: principal.subject,
        display_name: principal.display_name,
        agent_id,
        agent_code,
        lifecycle_status,
        active_workspace,
        permissions,
        locale: "id-ID".to_string(),
        timezone: "Asia/Jakarta".to_string(),
        session_expires_at: Some(Utc::now() + chrono::Duration::hours(8)),
        partial,
        unavailable_sections,
    };

    Ok(FeResponse::ok(
        profile,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/me/workspaces — IDENTITY-02
pub async fn get_workspaces(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<WorkspaceListResponse>, FeErrorResponse> {
    let permitted = resolve_permitted_workspaces(&state, &principal).await;
    let active = principal
        .workspace_id
        .clone()
        .filter(|id| permitted.iter().any(|w| &w.id == id))
        .or_else(|| permitted.first().map(|w| w.id.clone()));

    Ok(FeResponse::ok(
        WorkspaceListResponse {
            workspaces: permitted,
            active_workspace_id: active,
        },
        principal.request_id,
        principal.correlation_id,
    ))
}

/// POST /api/v1/me/workspaces/select — IDENTITY-03
pub async fn select_workspace(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(payload): Json<SelectWorkspaceRequest>,
) -> Result<FeResponse<serde_json::Value>, FeErrorResponse> {
    if payload.workspace_id.trim().is_empty() {
        return Err(FeErrorResponse::invalid_request(
            "workspaceId cannot be empty",
            principal.request_id,
            principal.correlation_id,
        ));
    }

    let permitted = resolve_permitted_workspaces(&state, &principal).await;
    if state.agency_client.is_some() && !permitted.iter().any(|w| w.id == payload.workspace_id) {
        return Err(FeErrorResponse::forbidden(
            format!(
                "User is not authorized for workspace '{}'",
                payload.workspace_id
            ),
            principal.request_id,
            principal.correlation_id,
        ));
    }

    // Update workspace in active session
    if let Ok(auth) = state.auth() {
        if let Some(mut session) = auth.store.get_session(&principal.session_id).await {
            session.workspace_id = Some(payload.workspace_id.clone());
            let _ = auth.store.update_session(&session).await;
        }
    }

    Ok(FeResponse::ok(
        serde_json::json!({
            "selectedWorkspaceId": payload.workspace_id,
            "switchedAt": Utc::now()
        }),
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/me/preferences — IDENTITY-04
pub async fn get_preferences(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<UserPreferencesDto>, FeErrorResponse> {
    let prefs = UserPreferencesDto {
        locale: "id-ID".to_string(),
        timezone: "Asia/Jakarta".to_string(),
        theme: "system".to_string(),
        notification_prefs: serde_json::json!({
            "email": true,
            "push": true,
            "sms": false
        }),
        version: 1,
    };

    Ok(FeResponse::ok(
        prefs,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// PATCH /api/v1/me/preferences — IDENTITY-05
pub async fn update_preferences(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(payload): Json<UpdatePreferencesRequest>,
) -> Result<FeResponse<UserPreferencesDto>, FeErrorResponse> {
    let updated = UserPreferencesDto {
        locale: payload.locale.unwrap_or_else(|| "id-ID".to_string()),
        timezone: payload
            .timezone
            .unwrap_or_else(|| "Asia/Jakarta".to_string()),
        theme: payload.theme.unwrap_or_else(|| "system".to_string()),
        notification_prefs: payload
            .notification_prefs
            .unwrap_or_else(|| serde_json::json!({})),
        version: payload.expected_version.unwrap_or(1) + 1,
    };

    Ok(FeResponse::ok(
        updated,
        principal.request_id,
        principal.correlation_id,
    ))
}
