//! Customer & Prospect endpoints (Group 6.10: CUSTOMERS-01 .. CUSTOMERS-03)
//!
//! Authoritative integration: downstream Customer Service required.
//! These endpoints are EXPLICITLY UNAVAILABLE until `iaop-customer-service`
//! is deployed and `CUSTOMER_SERVICE_URL` is configured.

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
pub struct CustomerDto {
    pub id: Uuid,
    pub full_name: String,
    pub identification_number: String, // Masked KTP
    pub phone: String,
    pub email: String,
    pub active_policies_count: u32,
    pub customer_since: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCustomerRequest {
    pub full_name: String,
    pub identification_number: String,
    pub phone: String,
    pub email: String,
    pub date_of_birth: String,
    pub address: String,
}

fn customers_unavailable(principal: &AuthenticatedPrincipal) -> FeErrorResponse {
    FeErrorResponse::new(
        CanonicalErrorCode::DependencyUnavailable,
        "Customer service is not configured. Downstream iaop-customer-service required.",
        principal.request_id,
        principal.correlation_id,
    )
}

/// GET /api/v1/customers — CUSTOMERS-01
pub async fn list_customers(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<CustomerDto>>, FeErrorResponse> {
    if state.config.customer_service_url.is_none() {
        return Err(customers_unavailable(&principal));
    }
    Err(customers_unavailable(&principal))
}

/// POST /api/v1/customers — CUSTOMERS-02
pub async fn create_customer(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(_payload): Json<CreateCustomerRequest>,
) -> Result<FeResponse<CustomerDto>, FeErrorResponse> {
    if state.config.customer_service_url.is_none() {
        return Err(customers_unavailable(&principal));
    }
    Err(customers_unavailable(&principal))
}

/// GET /api/v1/customers/{customerId} — CUSTOMERS-03
pub async fn get_customer(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(_customer_id): Path<Uuid>,
) -> Result<FeResponse<CustomerDto>, FeErrorResponse> {
    if state.config.customer_service_url.is_none() {
        return Err(customers_unavailable(&principal));
    }
    Err(customers_unavailable(&principal))
}
