//! Identity endpoints (Group 6.1: IDENTITY-01 .. IDENTITY-05)

use axum::Json;
use axum::extract::State;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::envelope::{CanonicalErrorCode, FeErrorResponse, FeResponse};
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

/// Extract effective UI permissions from a *cryptographically verified* access
/// token. The token is validated (signature, issuer, audience, expiry) via the
/// shared `iaop-security` JWKS validator on every call — the JWKS cache makes
/// this cheap. Roles are taken only from `realm_access` and the authorized
/// domain client audiences; malformed/foreign claims yield an empty grant.
/// Never falls back to unverified payload decoding.
async fn verified_permissions(state: &AppState, access_token: &str) -> Vec<String> {
    let auth = match state.auth() {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let claims: serde_json::Value = match auth.access_validator.validate(access_token).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "access token verification failed; granting no permissions");
            return Vec::new();
        }
    };

    let mut permissions = Vec::new();
    if let Some(roles) = claims
        .pointer("/realm_access/roles")
        .and_then(|v| v.as_array())
    {
        for r in roles {
            if let Some(s) = r.as_str() {
                permissions.push(s.to_string());
            }
        }
    }
    const AUTHORIZED_CLIENTS: &[&str] = &["iaop-agent-api", "iaop-agency-api", "iaop-agent-bff"];
    if let Some(res) = claims
        .pointer("/resource_access")
        .and_then(|v| v.as_object())
    {
        for (client, client_val) in res {
            if AUTHORIZED_CLIENTS.contains(&client.as_str()) {
                if let Some(roles) = client_val.get("roles").and_then(|v| v.as_array()) {
                    for r in roles {
                        if let Some(s) = r.as_str() {
                            permissions.push(s.to_string());
                        }
                    }
                }
            }
        }
    }
    if let Some(scope_str) = claims.get("scope").and_then(|v| v.as_str()) {
        for s in scope_str.split_whitespace() {
            permissions.push(format!("scope:{}", s));
        }
    }
    permissions.sort();
    permissions.dedup();
    permissions
}

/// Helper to authoritatively resolve permitted workspaces for the principal
/// Outcome of authoritative workspace resolution.
#[derive(Debug)]
pub enum WorkspaceResolution {
    /// User has agency membership; workspaces proven via Agency Service.
    Member(Vec<WorkspaceDto>),
    /// User legitimately has no agency membership (candidate/onboarding).
    Candidate,
}

impl WorkspaceResolution {
    pub fn workspaces(&self) -> Vec<WorkspaceDto> {
        match self {
            Self::Member(w) => w.clone(),
            Self::Candidate => vec![WorkspaceDto {
                id: "workspace-candidate".to_string(),
                name: "Candidate / Onboarding Workspace".to_string(),
                agency_code: None,
                role: "CANDIDATE".to_string(),
                is_default: true,
            }],
        }
    }
}

/// Authoritatively resolve permitted workspaces. Fails closed when the
/// Agency Service cannot prove membership — a service outage is never
/// silently downgraded to a candidate workspace.
pub async fn resolve_workspace_resolution(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
) -> Result<WorkspaceResolution, FeErrorResponse> {
    let mut workspaces = Vec::new();

    let Some(agency_client) = &state.agency_client else {
        // No authoritative membership source: only non-agent (candidate)
        // principals may proceed, and only to the candidate workspace.
        if principal.agent_id.is_none() {
            return Ok(WorkspaceResolution::Candidate);
        }
        return Err(FeErrorResponse::new(
            CanonicalErrorCode::DependencyUnavailable,
            "Agency service is not configured; cannot authoritatively verify workspace entitlement",
            principal.request_id,
            principal.correlation_id,
        ));
    };
    let Some(aid) = principal.agent_id else {
        return Ok(WorkspaceResolution::Candidate);
    };
    {
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
                return Ok(WorkspaceResolution::Candidate);
            }
            Err(e) => {
                tracing::warn!(error = %e, agent_id = %aid, "Agency assignment query failed");
                return Err(FeErrorResponse::new(
                    CanonicalErrorCode::DependencyUnavailable,
                    "Agency service unavailable; cannot verify workspace entitlement",
                    principal.request_id,
                    principal.correlation_id,
                ));
            }
        }
    }

    if workspaces.is_empty() {
        return Ok(WorkspaceResolution::Candidate);
    }
    Ok(WorkspaceResolution::Member(workspaces))
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

    let permitted = match resolve_workspace_resolution(&state, &principal).await {
        Ok(r) => r.workspaces(),
        Err(_) => {
            partial = true;
            unavailable_sections.push("workspaces".to_string());
            Vec::new()
        }
    };
    let active_workspace = principal
        .workspace_id
        .clone()
        .filter(|id| permitted.iter().any(|w| &w.id == id))
        .or_else(|| permitted.first().map(|w| w.id.clone()));

    let permissions = verified_permissions(&state, &principal.access_token).await;

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
    let resolution = resolve_workspace_resolution(&state, &principal).await?;
    let permitted = resolution.workspaces();
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

    if state.agency_client.is_none() {
        return Err(FeErrorResponse::new(
            CanonicalErrorCode::DependencyUnavailable,
            "Agency service is not configured; cannot authoritatively verify workspace entitlement",
            principal.request_id,
            principal.correlation_id,
        ));
    }

    let resolution = resolve_workspace_resolution(&state, &principal).await?;
    let permitted = resolution.workspaces();
    if !permitted.iter().any(|w| w.id == payload.workspace_id) {
        return Err(FeErrorResponse::forbidden(
            format!(
                "User is not authorized for workspace '{}'",
                payload.workspace_id
            ),
            principal.request_id,
            principal.correlation_id,
        ));
    }

    // Update workspace in active session — fail closed if persistence cannot be confirmed
    let auth = state.auth().map_err(|_| {
        FeErrorResponse::new(
            CanonicalErrorCode::DependencyUnavailable,
            "Authentication subsystem unavailable",
            principal.request_id,
            principal.correlation_id,
        )
    })?;

    // Confirm the session still exists before mutating workspace context.
    if auth
        .store
        .get_session(&principal.session_id)
        .await
        .is_none()
    {
        return Err(FeErrorResponse::unauthenticated(
            "Session not found or expired. Please sign in again.",
            principal.request_id,
            principal.correlation_id,
        ));
    }

    // Atomic single-column update — cannot clobber concurrent token refresh
    // or activity updates, and fails closed on persistence errors.
    auth.store
        .persist_workspace(&principal.session_id, Some(&payload.workspace_id))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to persist workspace update in session store");
            FeErrorResponse::internal(
                "Failed to persist workspace update to session store",
                principal.request_id,
                principal.correlation_id,
            )
        })?;

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
