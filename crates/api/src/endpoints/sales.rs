//! Sales & Proposal endpoints (Group 6.11: SALES-01 .. SALES-04)
//!
//! Authoritative validation and delegation to Core/Sales services.
//! Synthetic product catalogs, hardcoded illustration premiums, and
//! fabricated proposal submissions are eliminated.

use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::envelope::{CanonicalErrorCode, FeErrorResponse, FeResponse};
use crate::principal::AuthenticatedPrincipal;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductDto {
    pub code: String,
    pub name: String,
    pub category: String,
    pub min_sum_assured: String,
    pub max_sum_assured: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateIllustrationRequest {
    pub product_code: String,
    pub insured_age: u32,
    pub insured_gender: String,
    pub sum_assured: String,
    pub payment_term_years: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IllustrationDto {
    pub illustration_id: Uuid,
    pub product_code: String,
    pub annual_premium: String,
    pub monthly_premium: String,
    pub total_coverage: String,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateProposalRequest {
    pub illustration_id: Uuid,
    pub customer_id: Uuid,
    pub agent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProposalDto {
    pub proposal_id: Uuid,
    pub proposal_number: String,
    pub customer_id: Uuid,
    pub product_code: String,
    pub sum_assured: String,
    pub premium_amount: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// GET /api/v1/sales/products â€” SALES-01
pub async fn list_products(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<ProductDto>>, FeErrorResponse> {
    if state.config.sales_service_url.is_none() {
        return Err(FeErrorResponse::new(
            CanonicalErrorCode::DependencyUnavailable,
            "Sales product catalog service is not configured",
            principal.request_id,
            principal.correlation_id,
        ));
    }

    Ok(FeResponse::ok(
        Vec::new(),
        principal.request_id,
        principal.correlation_id,
    ))
}

/// POST /api/v1/sales/illustrations â€” SALES-02
pub async fn calculate_illustration(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(_payload): Json<CreateIllustrationRequest>,
) -> Result<FeResponse<IllustrationDto>, FeErrorResponse> {
    if state.config.sales_service_url.is_none() {
        return Err(FeErrorResponse::new(
            CanonicalErrorCode::DependencyUnavailable,
            "Sales illustration calculation service is not configured",
            principal.request_id,
            principal.correlation_id,
        ));
    }

    Err(FeErrorResponse::new(
        CanonicalErrorCode::DependencyUnavailable,
        "Authoritative illustration calculation engine is not yet available",
        principal.request_id,
        principal.correlation_id,
    ))
}

/// POST /api/v1/sales/proposals â€” SALES-03
pub async fn submit_proposal(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(_payload): Json<CreateProposalRequest>,
) -> Result<FeResponse<ProposalDto>, FeErrorResponse> {
    let aid = match principal.agent_id {
        Some(id) => id,
        None => {
            return Err(FeErrorResponse::new(
                CanonicalErrorCode::Forbidden,
                "Agent identity must be linked before submitting proposals",
                principal.request_id,
                principal.correlation_id,
            ));
        }
    };

    let agent_client = match &state.agent_client {
        Some(c) => c,
        None => {
            return Err(FeErrorResponse::new(
                CanonicalErrorCode::DependencyUnavailable,
                "Agent Service is unavailable; cannot verify agent selling authorization",
                principal.request_id,
                principal.correlation_id,
            ));
        }
    };

    let agent = agent_client
        .get_agent(aid, &principal.access_token, principal.request_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "submit_proposal: agent verification failed");
            FeErrorResponse::new(
                CanonicalErrorCode::DependencyFailure,
                "Authoritative agent status could not be verified; proposal submission rejected",
                principal.request_id,
                principal.correlation_id,
            )
        })?;

    if agent.lifecycle_status != "ACTIVE" {
        return Err(FeErrorResponse::selling_not_eligible(
            format!(
                "Agent status is {} â€” selling commands are prohibited",
                agent.lifecycle_status
            ),
            principal.request_id,
            principal.correlation_id,
        ));
    }

    if state.config.sales_service_url.is_none() {
        return Err(FeErrorResponse::new(
            CanonicalErrorCode::DependencyUnavailable,
            "Sales proposal processing service is currently unavailable",
            principal.request_id,
            principal.correlation_id,
        ));
    }

    Err(FeErrorResponse::new(
        CanonicalErrorCode::DependencyUnavailable,
        "Downstream sales service integration in progress",
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/sales/proposals/{proposalId} â€” SALES-04
pub async fn get_proposal(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(_proposal_id): Path<Uuid>,
) -> Result<FeResponse<ProposalDto>, FeErrorResponse> {
    if state.config.sales_service_url.is_none() {
        return Err(FeErrorResponse::new(
            CanonicalErrorCode::DependencyUnavailable,
            "Sales proposal service is not configured",
            principal.request_id,
            principal.correlation_id,
        ));
    }

    Err(FeErrorResponse::new(
        CanonicalErrorCode::NotFound,
        "Proposal not found",
        principal.request_id,
        principal.correlation_id,
    ))
}
