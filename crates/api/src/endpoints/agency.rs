//! Agency hierarchy & team endpoints (Group 6.9: AGENCY-01 .. AGENCY-02)

use axum::extract::State;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::envelope::{FeErrorResponse, FeResponse};
use crate::principal::AuthenticatedPrincipal;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgencyHierarchyNodeDto {
    pub agency_id: String,
    pub agency_name: String,
    pub leader_name: String,
    pub level: String, // "GA", "UM", "AGENT"
    pub parent_agency_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemberDto {
    pub agent_id: Uuid,
    pub agent_code: String,
    pub full_name: String,
    pub role: String,
    pub lifecycle_status: String,
    pub active_proposals_count: u32,
}

/// GET /api/v1/agency/hierarchy — AGENCY-01
pub async fn get_hierarchy(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<AgencyHierarchyNodeDto>>, FeErrorResponse> {
    let hierarchy = vec![
        AgencyHierarchyNodeDto {
            agency_id: "GA-001".to_string(),
            agency_name: "General Agency Nusantara Prima".to_string(),
            leader_name: "Bambang Soedirjo".to_string(),
            level: "GA".to_string(),
            parent_agency_id: None,
        },
        AgencyHierarchyNodeDto {
            agency_id: "UM-012".to_string(),
            agency_name: "Unit Mandiri Jakarta Barat".to_string(),
            leader_name: "Siti Rahmawati".to_string(),
            level: "UM".to_string(),
            parent_agency_id: Some("GA-001".to_string()),
        },
    ];

    Ok(FeResponse::ok(
        hierarchy,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/agency/my-team — AGENCY-02
pub async fn get_my_team(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<TeamMemberDto>>, FeErrorResponse> {
    let team = vec![TeamMemberDto {
        agent_id: principal.agent_id.unwrap_or_else(Uuid::now_v7),
        agent_code: "AGT-JKT001".to_string(),
        full_name: principal
            .display_name
            .unwrap_or_else(|| "Self Agent".to_string()),
        role: "PRODUCING_AGENT".to_string(),
        lifecycle_status: "ACTIVE".to_string(),
        active_proposals_count: 5,
    }];

    Ok(FeResponse::ok(
        team,
        principal.request_id,
        principal.correlation_id,
    ))
}
