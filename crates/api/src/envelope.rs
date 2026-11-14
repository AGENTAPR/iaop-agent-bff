//! Approved Frontend API Response Envelope.
//!
//! Conforms strictly to AGENA_FRONTEND_API_CONTRACT_V1 specification:
//!
//! Success:
//! ```json
//! {
//!   "data": {},
//!   "meta": {
//!     "requestId": "...",
//!     "correlationId": "...",
//!     "servedAt": "2026-11-14T03:00:00Z",
//!     "schemaVersion": "1.0"
//!   }
//! }
//! ```
//!
//! Error:
//! ```json
//! {
//!   "error": {
//!     "code": "VALIDATION_FAILED",
//!     "message": "...",
//!     "fieldErrors": [],
//!     "retryable": false,
//!     "operationId": null
//!   },
//!   "meta": { ... }
//! }
//! ```

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Envelope metadata included in every success and error response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResponseMeta {
    pub request_id: String,
    pub correlation_id: String,
    pub served_at: DateTime<Utc>,
    pub schema_version: String,
}

impl ResponseMeta {
    pub fn new(request_id: Option<Uuid>, correlation_id: Option<Uuid>) -> Self {
        let req_str = request_id
            .map(|u| u.to_string())
            .unwrap_or_else(|| Uuid::now_v7().to_string());
        let corr_str = correlation_id
            .map(|u| u.to_string())
            .unwrap_or_else(|| req_str.clone());
        Self {
            request_id: req_str,
            correlation_id: corr_str,
            served_at: Utc::now(),
            schema_version: "1.0".to_string(),
        }
    }
}

/// Canonical success envelope wrapping any data payload `T`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeResponse<T> {
    pub data: T,
    pub meta: ResponseMeta,
}

impl<T: Serialize> FeResponse<T> {
    pub fn ok(data: T, request_id: Option<Uuid>, correlation_id: Option<Uuid>) -> Self {
        Self {
            data,
            meta: ResponseMeta::new(request_id, correlation_id),
        }
    }
}

impl<T: Serialize> IntoResponse for FeResponse<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Individual field-level error detail.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FieldError {
    pub field: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Canonical error codes mandated by IAOP API standards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalErrorCode {
    InvalidRequest,
    Unauthenticated,
    Forbidden,
    NotFound,
    VersionConflict,
    IdempotencyKeyReused,
    ValidationFailed,
    SellingNotEligible,
    ResourceLocked,
    RateLimited,
    DependencyFailure,
    DependencyUnavailable,
    InternalError,
}

impl CanonicalErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidRequest => "INVALID_REQUEST",
            Self::Unauthenticated => "UNAUTHENTICATED",
            Self::Forbidden => "FORBIDDEN",
            Self::NotFound => "NOT_FOUND",
            Self::VersionConflict => "VERSION_CONFLICT",
            Self::IdempotencyKeyReused => "IDEMPOTENCY_KEY_REUSED",
            Self::ValidationFailed => "VALIDATION_FAILED",
            Self::SellingNotEligible => "SELLING_NOT_ELIGIBLE",
            Self::ResourceLocked => "RESOURCE_LOCKED",
            Self::RateLimited => "RATE_LIMITED",
            Self::DependencyFailure => "DEPENDENCY_FAILURE",
            Self::DependencyUnavailable => "DEPENDENCY_UNAVAILABLE",
            Self::InternalError => "INTERNAL_ERROR",
        }
    }

    pub fn http_status(&self) -> StatusCode {
        match self {
            Self::InvalidRequest => StatusCode::BAD_REQUEST,
            Self::Unauthenticated => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::VersionConflict => StatusCode::CONFLICT,
            Self::IdempotencyKeyReused => StatusCode::CONFLICT,
            Self::ValidationFailed => StatusCode::UNPROCESSABLE_ENTITY,
            Self::SellingNotEligible => StatusCode::FORBIDDEN,
            Self::ResourceLocked => StatusCode::LOCKED,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::DependencyFailure => StatusCode::BAD_GATEWAY,
            Self::DependencyUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn default_retryable(&self) -> bool {
        matches!(self, Self::RateLimited | Self::DependencyUnavailable)
    }
}

/// Error body structure inside the canonical error envelope.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FeErrorBody {
    pub code: String,
    pub message: String,
    pub field_errors: Vec<FieldError>,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
}

/// Canonical error envelope.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeErrorResponse {
    pub error: Box<FeErrorBody>,
    pub meta: Box<ResponseMeta>,
    #[serde(skip)]
    status: StatusCode,
}

impl FeErrorResponse {
    pub fn new(
        code: CanonicalErrorCode,
        message: impl Into<String>,
        request_id: Option<Uuid>,
        correlation_id: Option<Uuid>,
    ) -> Self {
        let status = code.http_status();
        let retryable = code.default_retryable();
        Self {
            status,
            error: Box::new(FeErrorBody {
                code: code.as_str().to_string(),
                message: message.into(),
                field_errors: Vec::new(),
                retryable,
                operation_id: None,
            }),
            meta: Box::new(ResponseMeta::new(request_id, correlation_id)),
        }
    }

    pub fn with_field_errors(mut self, errors: Vec<FieldError>) -> Self {
        self.error.field_errors = errors;
        self
    }

    pub fn with_operation_id(mut self, op_id: impl Into<String>) -> Self {
        self.error.operation_id = Some(op_id.into());
        self
    }

    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    pub fn unauthenticated(msg: impl Into<String>, req: Option<Uuid>, corr: Option<Uuid>) -> Self {
        Self::new(CanonicalErrorCode::Unauthenticated, msg, req, corr)
    }

    pub fn forbidden(msg: impl Into<String>, req: Option<Uuid>, corr: Option<Uuid>) -> Self {
        Self::new(CanonicalErrorCode::Forbidden, msg, req, corr)
    }

    pub fn not_found(msg: impl Into<String>, req: Option<Uuid>, corr: Option<Uuid>) -> Self {
        Self::new(CanonicalErrorCode::NotFound, msg, req, corr)
    }

    pub fn validation(
        msg: impl Into<String>,
        fields: Vec<FieldError>,
        req: Option<Uuid>,
        corr: Option<Uuid>,
    ) -> Self {
        Self::new(CanonicalErrorCode::ValidationFailed, msg, req, corr).with_field_errors(fields)
    }

    pub fn dependency_unavailable(
        msg: impl Into<String>,
        req: Option<Uuid>,
        corr: Option<Uuid>,
    ) -> Self {
        Self::new(CanonicalErrorCode::DependencyUnavailable, msg, req, corr)
    }

    pub fn dependency_failure(
        msg: impl Into<String>,
        req: Option<Uuid>,
        corr: Option<Uuid>,
    ) -> Self {
        Self::new(CanonicalErrorCode::DependencyFailure, msg, req, corr)
    }

    pub fn invalid_request(msg: impl Into<String>, req: Option<Uuid>, corr: Option<Uuid>) -> Self {
        Self::new(CanonicalErrorCode::InvalidRequest, msg, req, corr)
    }

    pub fn selling_not_eligible(
        msg: impl Into<String>,
        req: Option<Uuid>,
        corr: Option<Uuid>,
    ) -> Self {
        Self::new(CanonicalErrorCode::SellingNotEligible, msg, req, corr)
    }

    pub fn conflict(msg: impl Into<String>, req: Option<Uuid>, corr: Option<Uuid>) -> Self {
        Self::new(CanonicalErrorCode::VersionConflict, msg, req, corr)
    }

    pub fn internal(msg: impl Into<String>, req: Option<Uuid>, corr: Option<Uuid>) -> Self {
        Self::new(CanonicalErrorCode::InternalError, msg, req, corr)
    }
}

impl IntoResponse for FeErrorResponse {
    fn into_response(self) -> Response {
        let status = self.status;
        (status, Json(self)).into_response()
    }
}
