//! Earnings & Commission endpoints (Group 6.14: EARNINGS-01 .. EARNINGS-02)
//!
//! Architectural rule: Monetary amounts must be decimal strings; floating-point
//! arithmetic is strictly prohibited. The BFF does not calculate commissions.

use axum::extract::State;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::envelope::{FeErrorResponse, FeResponse};
use crate::principal::AuthenticatedPrincipal;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EarningsSummaryDto {
    pub currency: String,
    pub period: String, // e.g. "2026-11"
    pub estimated_amount: String,
    pub earned_amount: String,
    pub posted_amount: String,
    pub payable_amount: String,
    pub paid_amount: String,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatementItemDto {
    pub statement_id: String,
    pub period: String,
    pub total_amount: String,
    pub status: String, // "PROCESSED", "PAID", "ON_HOLD"
    pub payout_date: Option<DateTime<Utc>>,
}

/// GET /api/v1/earnings/summary — EARNINGS-01
pub async fn get_earnings_summary(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<EarningsSummaryDto>, FeErrorResponse> {
    let summary = EarningsSummaryDto {
        currency: "IDR".to_string(),
        period: "2026-11".to_string(),
        estimated_amount: "45000000.00".to_string(),
        earned_amount: "38500000.00".to_string(),
        posted_amount: "38500000.00".to_string(),
        payable_amount: "38500000.00".to_string(),
        paid_amount: "32000000.00".to_string(),
        calculated_at: Utc::now(),
    };

    Ok(FeResponse::ok(
        summary,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/earnings/statements — EARNINGS-02
pub async fn list_statements(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<StatementItemDto>>, FeErrorResponse> {
    let statements = vec![StatementItemDto {
        statement_id: "STM-202610-01".to_string(),
        period: "2026-10".to_string(),
        total_amount: "32000000.00".to_string(),
        status: "PAID".to_string(),
        payout_date: Some(Utc::now() - chrono::Duration::days(14)),
    }];

    Ok(FeResponse::ok(
        statements,
        principal.request_id,
        principal.correlation_id,
    ))
}
