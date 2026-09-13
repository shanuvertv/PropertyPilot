//! Maps `ServiceError` onto HTTP statuses and the `{ "error": {...} }` body from `renewal_api`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use renewal_api::{ApiError, ErrorBody, ErrorCode};
use renewal_db::sqlx;
use renewal_services::ServiceError;

#[derive(Debug)]
pub struct ApiFailure(pub ServiceError);

/// HTTP-only failure: the login limiter tripped (no service-layer equivalent).
#[derive(Debug)]
pub struct RateLimited;

impl IntoResponse for RateLimited {
    fn into_response(self) -> Response {
        (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorBody {
                error: ApiError {
                    code: ErrorCode::RateLimited,
                    message: "too many sign-in attempts; try again in 15 minutes".to_owned(),
                },
            }),
        )
            .into_response()
    }
}

/// Either way `login` can fail. An enum (not a ready-made `Response`) keeps the
/// handler's `Err` small — clippy's `result_large_err` rejects a 128-byte `Response`.
#[derive(Debug)]
pub enum LoginFailure {
    RateLimited,
    Api(ApiFailure),
}

impl From<ServiceError> for LoginFailure {
    fn from(e: ServiceError) -> Self {
        LoginFailure::Api(ApiFailure(e))
    }
}

impl IntoResponse for LoginFailure {
    fn into_response(self) -> Response {
        match self {
            LoginFailure::RateLimited => RateLimited.into_response(),
            LoginFailure::Api(e) => e.into_response(),
        }
    }
}

impl From<ServiceError> for ApiFailure {
    fn from(e: ServiceError) -> Self {
        ApiFailure(e)
    }
}

impl From<sqlx::Error> for ApiFailure {
    fn from(e: sqlx::Error) -> Self {
        ApiFailure(ServiceError::Db(e))
    }
}

impl ApiFailure {
    pub fn unauthorized() -> Self {
        ApiFailure(ServiceError::Unauthenticated)
    }
}

impl IntoResponse for ApiFailure {
    fn into_response(self) -> Response {
        use ServiceError::*;
        let (status, code, message) = match &self.0 {
            Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                ErrorCode::Unauthorized,
                "sign in to continue".to_owned(),
            ),
            PermissionDenied(e) => (StatusCode::FORBIDDEN, ErrorCode::Forbidden, e.to_string()),
            InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                ErrorCode::InvalidCredentials,
                self.0.to_string(),
            ),
            Validation(m) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                ErrorCode::Validation,
                m.clone(),
            ),
            Conflict(m) => (StatusCode::CONFLICT, ErrorCode::Conflict, m.clone()),
            NotFound(_) => (
                StatusCode::NOT_FOUND,
                ErrorCode::NotFound,
                self.0.to_string(),
            ),
            Transition(e) => (StatusCode::CONFLICT, ErrorCode::Conflict, e.to_string()),
            Db(e) => {
                tracing::error!(error = %e, "database error");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    ErrorCode::ServiceUnavailable,
                    "database error; try again".to_owned(),
                )
            }
            Internal(m) => {
                tracing::error!(error = %m, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorCode::Internal,
                    "internal error".to_owned(),
                )
            }
        };
        (
            status,
            Json(ErrorBody {
                error: ApiError { code, message },
            }),
        )
            .into_response()
    }
}
