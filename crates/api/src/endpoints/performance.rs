//! Performance insight endpoints (Group 6.15: PERFORMANCE-01 .. PERFORMANCE-02)

use axum::extract::State;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::envelope::{FeErrorResponse, FeResponse};
use crate::principal::AuthenticatedPrincipal;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceMetricDto {
    pub metric_code: String,
    pub name: String,
    pub current_value: String,
    pub target_value: String,
    pub unit: String,
    pub achievement_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceOverviewDto {
    pub period: String,
    pub tier: String, // "SILVER", "GOLD", "PLATINUM", "MDRT"
    pub mdr_progress_percentage: f64,
    pub metrics: Vec<PerformanceMetricDto>,
    pub as_of: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardRankDto {
    pub rank: u32,
    pub agent_name: String,
    pub agency_name: String,
    pub score: String,
    pub is_current_user: bool,
}

/// GET /api/v1/performance/metrics — PERFORMANCE-01
pub async fn get_performance_metrics(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<PerformanceOverviewDto>, FeErrorResponse> {
    let metrics = vec![
        PerformanceMetricDto {
            metric_code: "APE_YTD".to_string(),
            name: "Annualized Premium Equivalent (APE)".to_string(),
            current_value: "450000000.00".to_string(),
            target_value: "600000000.00".to_string(),
            unit: "IDR".to_string(),
            achievement_percentage: 75.0,
        },
        PerformanceMetricDto {
            metric_code: "CASES_COUNT".to_string(),
            name: "Jumlah Polis Terbit".to_string(),
            current_value: "24".to_string(),
            target_value: "30".to_string(),
            unit: "KASUS".to_string(),
            achievement_percentage: 80.0,
        },
    ];

    let overview = PerformanceOverviewDto {
        period: "2026-YTD".to_string(),
        tier: "GOLD".to_string(),
        mdr_progress_percentage: 72.5,
        metrics,
        as_of: Utc::now(),
    };

    Ok(FeResponse::ok(
        overview,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/performance/rankings — PERFORMANCE-02
pub async fn get_rankings(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<LeaderboardRankDto>>, FeErrorResponse> {
    let ranks = vec![
        LeaderboardRankDto {
            rank: 1,
            agent_name: "Maya Indira".to_string(),
            agency_name: "GA Nusantara Prima".to_string(),
            score: "850000000.00".to_string(),
            is_current_user: false,
        },
        LeaderboardRankDto {
            rank: 8,
            agent_name: principal
                .display_name
                .unwrap_or_else(|| "Self Agent".to_string()),
            agency_name: "GA Nusantara Prima".to_string(),
            score: "450000000.00".to_string(),
            is_current_user: true,
        },
    ];

    Ok(FeResponse::ok(
        ranks,
        principal.request_id,
        principal.correlation_id,
    ))
}
