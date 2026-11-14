//! Customer & Prospect endpoints (Group 6.10: CUSTOMERS-01 .. CUSTOMERS-03)

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

/// GET /api/v1/customers — CUSTOMERS-01
pub async fn list_customers(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<CustomerDto>>, FeErrorResponse> {
    let customers = vec![CustomerDto {
        id: Uuid::now_v7(),
        full_name: "Hendra Wijaya".to_string(),
        identification_number: "3171************".to_string(),
        phone: "+628123456789".to_string(),
        email: "hendra.w@example.com".to_string(),
        active_policies_count: 1,
        customer_since: Utc::now() - chrono::Duration::days(180),
    }];

    Ok(FeResponse::ok(
        customers,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// POST /api/v1/customers — CUSTOMERS-02
pub async fn create_customer(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(payload): Json<CreateCustomerRequest>,
) -> Result<FeResponse<CustomerDto>, FeErrorResponse> {
    if payload.full_name.trim().is_empty() || payload.identification_number.trim().is_empty() {
        return Err(FeErrorResponse::validation(
            "Customer name and identification number are required",
            vec![],
            principal.request_id,
            principal.correlation_id,
        ));
    }

    let customer = CustomerDto {
        id: Uuid::now_v7(),
        full_name: payload.full_name,
        identification_number: format!(
            "{}************",
            &payload.identification_number[..payload.identification_number.len().min(4)]
        ),
        phone: payload.phone,
        email: payload.email,
        active_policies_count: 0,
        customer_since: Utc::now(),
    };

    Ok(FeResponse::ok(
        customer,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/customers/{customerId} — CUSTOMERS-03
pub async fn get_customer(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(customer_id): Path<Uuid>,
) -> Result<FeResponse<CustomerDto>, FeErrorResponse> {
    let customer = CustomerDto {
        id: customer_id,
        full_name: "Hendra Wijaya".to_string(),
        identification_number: "3171************".to_string(),
        phone: "+628123456789".to_string(),
        email: "hendra.w@example.com".to_string(),
        active_policies_count: 1,
        customer_since: Utc::now() - chrono::Duration::days(180),
    };

    Ok(FeResponse::ok(
        customer,
        principal.request_id,
        principal.correlation_id,
    ))
}
