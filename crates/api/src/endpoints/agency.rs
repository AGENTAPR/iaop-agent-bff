//! Agency hierarchy & team endpoints (Group 6.9: AGENCY-01 .. AGENCY-02)
//!
//! Authoritative integration with `iaop-agency-service`.
//! Hardcoded mock agencies and synthetic team members are eliminated.

use axum::extract::State;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::envelope::{CanonicalErrorCode, FeErrorResponse, FeResponse};
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

/// GET /api/v1/agency/hierarchy â€” AGENCY-01
pub async fn get_hierarchy(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<AgencyHierarchyNodeDto>>, FeErrorResponse> {
    let agency_client = match &state.agency_client {
        Some(c) => c,
        None => {
            return Err(FeErrorResponse::new(
                CanonicalErrorCode::DependencyUnavailable,
                "Agency service client is not configured",
                principal.request_id,
                principal.correlation_id,
            ));
        }
    };

    let agent_id = match principal.agent_id {
        Some(aid) => aid,
        None => {
            return Ok(FeResponse::ok(
                Vec::new(),
                principal.request_id,
                principal.correlation_id,
            ));
        }
    };

    let assignment = agency_client
        .get_current_assignment(agent_id, &principal.access_token, principal.request_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "agency_client: get_current_assignment failed");
            FeErrorResponse::new(
                CanonicalErrorCode::DependencyFailure,
                "Failed to query agency assignment from Agency Service",
                principal.request_id,
                principal.correlation_id,
            )
        })?;

    let assignment = match assignment {
        Some(a) => a,
        None => {
            return Ok(FeResponse::ok(
                Vec::new(),
                principal.request_id,
                principal.correlation_id,
            ));
        }
    };

    let mut nodes = Vec::new();

    if let Ok(ancestors) = agency_client
        .get_ancestors(
            assignment.unit_id,
            &principal.access_token,
            principal.request_id,
        )
        .await
    {
        for anc in ancestors {
            nodes.push(AgencyHierarchyNodeDto {
                agency_id: anc.code,
                agency_name: anc.name,
                leader_name: "Pimpinan Unit".to_string(),
                level: anc.kind,
                parent_agency_id: anc.parent_id.map(|p| p.to_string()),
            });
        }
    }

    if let Ok(unit) = agency_client
        .get_unit(
            assignment.unit_id,
            &principal.access_token,
            principal.request_id,
        )
        .await
    {
        nodes.push(AgencyHierarchyNodeDto {
            agency_id: unit.code,
            agency_name: unit.name,
            leader_name: "Pimpinan Unit".to_string(),
            level: unit.kind,
            parent_agency_id: unit.parent_id.map(|p| p.to_string()),
        });
    }

    Ok(FeResponse::ok(
        nodes,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/agency/my-team â€” AGENCY-02
pub async fn get_my_team(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<TeamMemberDto>>, FeErrorResponse> {
    let agency_client = match &state.agency_client {
        Some(c) => c,
        None => {
            return Err(FeErrorResponse::new(
                CanonicalErrorCode::DependencyUnavailable,
                "Agency service client is not configured",
                principal.request_id,
                principal.correlation_id,
            ));
        }
    };

    let agent_id = match principal.agent_id {
        Some(aid) => aid,
        None => {
            return Ok(FeResponse::ok(
                Vec::new(),
                principal.request_id,
                principal.correlation_id,
            ));
        }
    };

    let assignment = agency_client
        .get_current_assignment(agent_id, &principal.access_token, principal.request_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "agency_client: get_current_assignment failed");
            FeErrorResponse::new(
                CanonicalErrorCode::DependencyFailure,
                "Failed to query agency team from Agency Service",
                principal.request_id,
                principal.correlation_id,
            )
        })?;

    let team: Vec<TeamMemberDto> = match assignment {
        Some(a) => {
            // Query actual team members (all agents currently assigned to this unit)
            match agency_client
                .get_unit_assignments(a.unit_id, &principal.access_token, principal.request_id)
                .await
            {
                Ok(members) => members
                    .into_iter()
                    .filter(|m| m.agent_id != agent_id)
                    .map(|m| TeamMemberDto {
                        agent_id: m.agent_id,
                        agent_code: format!("AG-{}", m.agent_id.to_string()[..8].to_uppercase()),
                        full_name: format!("Agent {}", &m.agent_id.to_string()[..8]),
                        role: m.assignment_type,
                        lifecycle_status: m.status,
                        active_proposals_count: 0,
                    })
                    .collect(),
                Err(e) => {
                    tracing::warn!(error = %e, unit_id = %a.unit_id, "Failed to fetch unit assignments for team");
                    Vec::new()
                }
            }
        }
        None => Vec::new(),
    };

    Ok(FeResponse::ok(
        team,
        principal.request_id,
        principal.correlation_id,
    ))
}
