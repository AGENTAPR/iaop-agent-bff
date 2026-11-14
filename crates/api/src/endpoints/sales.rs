//! Sales & Proposal endpoints (Group 6.11: SALES-01 .. SALES-04)

use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::envelope::{FeErrorResponse, FeResponse};
use crate::principal::AuthenticatedPrincipal;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductDto {
    pub code: String,
    pub name: String,
    pub category: String, // "TERM_LIFE", "WHOLE_LIFE", "HEALTH", "CRITICAL_ILLNESS"
    pub min_sum_assured: String, // Decimal string as required by monetary standard
    pub max_sum_assured: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateIllustrationRequest {
    pub product_code: String,
    pub insured_age: u32,
    pub insured_gender: String,
    pub sum_assured: String, // Decimal string
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
    pub status: String, // "DRAFT", "SUBMITTED", "UNDERWRITING", "ACCEPTED", "DECLINED"
    pub created_at: DateTime<Utc>,
}

/// GET /api/v1/sales/products — SALES-01
pub async fn list_products(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<ProductDto>>, FeErrorResponse> {
    let products = vec![
        ProductDto {
            code: "PRD-TL-01".to_string(),
            name: "Nusantara Term Life Proteksi Prima".to_string(),
            category: "TERM_LIFE".to_string(),
            min_sum_assured: "100000000.00".to_string(),
            max_sum_assured: "5000000000.00".to_string(),
            is_active: true,
        },
        ProductDto {
            code: "PRD-HLT-02".to_string(),
            name: "Nusantara Health Sejahtera Plus".to_string(),
            category: "HEALTH".to_string(),
            min_sum_assured: "500000000.00".to_string(),
            max_sum_assured: "20000000000.00".to_string(),
            is_active: true,
        },
    ];

    Ok(FeResponse::ok(
        products,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// POST /api/v1/sales/illustrations — SALES-02
pub async fn calculate_illustration(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(payload): Json<CreateIllustrationRequest>,
) -> Result<FeResponse<IllustrationDto>, FeErrorResponse> {
    let illustration = IllustrationDto {
        illustration_id: Uuid::now_v7(),
        product_code: payload.product_code,
        annual_premium: "12000000.00".to_string(),
        monthly_premium: "1050000.00".to_string(),
        total_coverage: payload.sum_assured,
        calculated_at: Utc::now(),
    };

    Ok(FeResponse::ok(
        illustration,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// POST /api/v1/sales/proposals — SALES-03
pub async fn submit_proposal(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(payload): Json<CreateProposalRequest>,
) -> Result<FeResponse<ProposalDto>, FeErrorResponse> {
    // Check selling eligibility if agent service is configured
    if let (Some(client), Some(aid)) = (&state.agent_client, principal.agent_id) {
        if let Ok(agent) = client
            .get_agent(aid, &principal.access_token, principal.request_id)
            .await
        {
            if agent.lifecycle_status != "ACTIVE" {
                return Err(FeErrorResponse::selling_not_eligible(
                    format!(
                        "Agent status is {} — selling commands are prohibited",
                        agent.lifecycle_status
                    ),
                    principal.request_id,
                    principal.correlation_id,
                ));
            }
        }
    }

    let proposal_id = Uuid::now_v7();
    let num = format!(
        "PROP-{}",
        proposal_id.simple().to_string()[..10].to_uppercase()
    );

    let proposal = ProposalDto {
        proposal_id,
        proposal_number: num,
        customer_id: payload.customer_id,
        product_code: "PRD-TL-01".to_string(),
        sum_assured: "500000000.00".to_string(),
        premium_amount: "12000000.00".to_string(),
        status: "SUBMITTED".to_string(),
        created_at: Utc::now(),
    };

    Ok(FeResponse::ok(
        proposal,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/sales/proposals/{proposalId} — SALES-04
pub async fn get_proposal(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(proposal_id): Path<Uuid>,
) -> Result<FeResponse<ProposalDto>, FeErrorResponse> {
    let num = format!(
        "PROP-{}",
        proposal_id.simple().to_string()[..10].to_uppercase()
    );
    let proposal = ProposalDto {
        proposal_id,
        proposal_number: num,
        customer_id: Uuid::now_v7(),
        product_code: "PRD-TL-01".to_string(),
        sum_assured: "500000000.00".to_string(),
        premium_amount: "12000000.00".to_string(),
        status: "SUBMITTED".to_string(),
        created_at: Utc::now() - chrono::Duration::hours(2),
    };

    Ok(FeResponse::ok(
        proposal,
        principal.request_id,
        principal.correlation_id,
    ))
}
