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
    /// Authoritative leader name — `None` until a leadership read is wired.
    pub leader_name: Option<String>,
    pub level: String, // "GA", "UM", "AGENT"
    pub parent_agency_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemberDto {
    pub agent_id: Uuid,
    /// Authoritative agent code; `None` when Agent Service is unreachable.
    pub agent_code: Option<String>,
    /// Authoritative display name; `None` — Agent Service exposes no name field yet.
    pub full_name: Option<String>,
    pub role: String,
    /// Authoritative lifecycle status; `None` when Agent Service is unreachable.
    pub lifecycle_status: Option<String>,
    /// Authoritative proposal count; `None` — no sales read model is wired yet.
    pub active_proposals_count: Option<u32>,
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

    // Upstream `ancestors` returns unit IDs only — resolve each to a full
    // unit via `get_unit` (each call is independently authorized upstream).
    if let Ok(ancestor_ids) = agency_client
        .get_ancestors(
            assignment.unit_id,
            &principal.access_token,
            principal.request_id,
        )
        .await
    {
        for anc_id in ancestor_ids {
            match agency_client
                .get_unit(anc_id, &principal.access_token, principal.request_id)
                .await
            {
                Ok(anc) => nodes.push(AgencyHierarchyNodeDto {
                    agency_id: anc.code,
                    agency_name: anc.name,
                    leader_name: None,
                    level: anc.kind,
                    parent_agency_id: anc.parent_id.map(|p| p.to_string()),
                }),
                Err(e) => {
                    tracing::warn!(error = %e, unit_id = %anc_id, "Failed to resolve ancestor unit");
                }
            }
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
            leader_name: None,
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
            let other_members: Vec<_> = match agency_client
                .get_unit_assignments(a.unit_id, &principal.access_token, principal.request_id)
                .await
            {
                Ok(members) => members
                    .into_iter()
                    .filter(|m| m.agent_id != agent_id)
                    .collect(),
                Err(e) => {
                    tracing::warn!(error = %e, unit_id = %a.unit_id, "Failed to fetch unit assignments for team");
                    Vec::new()
                }
            };

            // Enrich member details authoritatively from Agent Service.
            // Fetches run concurrently (bounded by member count); a failed
            // lookup yields explicit `None` fields — never fabricated values.
            let mut enriched_team = Vec::with_capacity(other_members.len());
            let mut set = tokio::task::JoinSet::new();
            for m in other_members {
                let client = state.agent_client.clone();
                let token = principal.access_token.clone();
                let rid = principal.request_id;
                set.spawn(async move {
                    let view = match client {
                        Some(c) => c.get_agent(m.agent_id, &token, rid).await.ok(),
                        None => None,
                    };
                    (m, view)
                });
            }
            while let Some(joined) = set.join_next().await {
                let Ok((m, view)) = joined else { continue };
                enriched_team.push(TeamMemberDto {
                    agent_id: m.agent_id,
                    agent_code: view.as_ref().map(|v| v.agent_code.clone()),
                    full_name: None,
                    role: m.assignment_type,
                    lifecycle_status: view.map(|v| v.lifecycle_status),
                    active_proposals_count: None,
                });
            }
            enriched_team
        }
        None => Vec::new(),
    };

    Ok(FeResponse::ok(
        team,
        principal.request_id,
        principal.correlation_id,
    ))
}
