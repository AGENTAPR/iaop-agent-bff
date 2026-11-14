//! Learning endpoints (Group 6.7: LEARNING-01 .. LEARNING-04)

use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::envelope::{FeErrorResponse, FeResponse};
use crate::principal::AuthenticatedPrincipal;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CourseDto {
    pub id: String,
    pub title: String,
    pub category: String, // "COMPLIANCE", "PRODUCT_KNOWLEDGE", "SALES_SKILLS"
    pub duration_minutes: u32,
    pub mandatory: bool,
    pub enrolled: bool,
    pub progress_percentage: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearningProgressSummaryDto {
    pub courses_completed: u32,
    pub courses_in_progress: u32,
    pub mandatory_completed: bool,
    pub last_activity_at: Option<DateTime<Utc>>,
}

/// GET /api/v1/learning/courses — LEARNING-01
pub async fn list_courses(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<Vec<CourseDto>>, FeErrorResponse> {
    let courses = vec![
        CourseDto {
            id: "crs-cpd-01".to_string(),
            title: "Kepatuhan Kode Etik & Regulasi OJK".to_string(),
            category: "COMPLIANCE".to_string(),
            duration_minutes: 120,
            mandatory: true,
            enrolled: true,
            progress_percentage: 100,
        },
        CourseDto {
            id: "crs-prod-02".to_string(),
            title: "Pengenalan Produk Asuransi Tradisional & Unit Link".to_string(),
            category: "PRODUCT_KNOWLEDGE".to_string(),
            duration_minutes: 90,
            mandatory: true,
            enrolled: true,
            progress_percentage: 75,
        },
    ];

    Ok(FeResponse::ok(
        courses,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/learning/courses/{courseId} — LEARNING-02
pub async fn get_course(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(course_id): Path<String>,
) -> Result<FeResponse<CourseDto>, FeErrorResponse> {
    let course = CourseDto {
        id: course_id,
        title: "Kepatuhan Kode Etik & Regulasi OJK".to_string(),
        category: "COMPLIANCE".to_string(),
        duration_minutes: 120,
        mandatory: true,
        enrolled: true,
        progress_percentage: 100,
    };

    Ok(FeResponse::ok(
        course,
        principal.request_id,
        principal.correlation_id,
    ))
}

/// POST /api/v1/learning/courses/{courseId}/enroll — LEARNING-03
pub async fn enroll_course(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(course_id): Path<String>,
) -> Result<FeResponse<serde_json::Value>, FeErrorResponse> {
    Ok(FeResponse::ok(
        serde_json::json!({
            "courseId": course_id,
            "enrolled": true,
            "enrolledAt": Utc::now()
        }),
        principal.request_id,
        principal.correlation_id,
    ))
}

/// GET /api/v1/learning/my-progress — LEARNING-04
pub async fn get_my_learning_progress(
    _state: State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<FeResponse<LearningProgressSummaryDto>, FeErrorResponse> {
    let progress = LearningProgressSummaryDto {
        courses_completed: 4,
        courses_in_progress: 1,
        mandatory_completed: true,
        last_activity_at: Some(Utc::now() - chrono::Duration::hours(5)),
    };

    Ok(FeResponse::ok(
        progress,
        principal.request_id,
        principal.correlation_id,
    ))
}
