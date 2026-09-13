//! Wire types for the HTTP API (`apps/server`) shared with every client.
//!
//! Mirrored by hand in `apps/desktop/src/api/types.ts` until the TypeScript
//! export is wired (Phase 1). Keep the two in sync — every field here is
//! camelCase on the wire.
//!
//! Conventions: ids and timestamps are strings (UUID / RFC 3339) on the wire.

use renewal_core::{Capability, Role, Thresholds};
use serde::{Deserialize, Serialize};

pub mod automation;
pub mod contracts;
pub mod email;
pub mod expenses;
pub mod master;

pub use automation::*;
pub use contracts::*;
pub use email::*;
pub use expenses::*;
pub use master::*;

pub const API_PREFIX: &str = "/api";

// ---------------------------------------------------------------- errors

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum ErrorCode {
    Unauthorized,
    Forbidden,
    NotFound,
    Validation,
    Conflict,
    InvalidCredentials,
    RateLimited,
    Internal,
    ServiceUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
}

/// Every non-2xx response body: `{ "error": { "code": ..., "message": ... } }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ErrorBody {
    pub error: ApiError,
}

// ---------------------------------------------------------------- auth

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SessionInfo {
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub role: Role,
    /// Everything this role may do (PLAN.md §6.7), so the UI never duplicates the matrix.
    pub capabilities: Vec<Capability>,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct LoginResponse {
    /// Opaque bearer token. Store it in the platform secure store, never in plain files.
    pub token: String,
    pub session: SessionInfo,
}

/// Creates the first Admin. Only accepted while the `users` table is empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BootstrapRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ResetPasswordRequest {
    pub new_password: String,
}

// ---------------------------------------------------------------- users

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct UserSummary {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: Role,
    pub active: bool,
    pub created_at: String,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub role: Role,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SetUserActiveRequest {
    pub active: bool,
}

// ---------------------------------------------------------------- system

/// Public (unauthenticated) probe used by the client before login.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct HealthResponse {
    pub ok: bool,
    pub version: String,
    pub database: bool,
    /// `false` until the first Admin is created — the client then shows the Bootstrap screen.
    pub users_exist: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SchedulerStatus {
    pub version: String,
    pub hostname: String,
    pub last_heartbeat_at: String,
    pub last_sweep_at: Option<String>,
    /// True when no heartbeat for more than `stale_after_minutes`.
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SystemStatus {
    pub org_name: String,
    pub timezone: String,
    pub thresholds: Thresholds,
    pub scheduler: Option<SchedulerStatus>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_format_is_camel_case_with_screaming_enums() {
        let body = ErrorBody {
            error: ApiError {
                code: ErrorCode::InvalidCredentials,
                message: "no".into(),
            },
        };
        assert_eq!(
            serde_json::to_string(&body).unwrap(),
            r#"{"error":{"code":"INVALID_CREDENTIALS","message":"no"}}"#
        );
        let s = SessionInfo {
            user_id: "u".into(),
            name: "n".into(),
            email: "e".into(),
            role: Role::Leasing,
            capabilities: vec![Capability::ViewDashboard],
            expires_at: "t".into(),
        };
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            r#"{"userId":"u","name":"n","email":"e","role":"LEASING","capabilities":["VIEW_DASHBOARD"],"expiresAt":"t"}"#
        );
    }
}
