use axum::extract::State;
use axum::Json;
use renewal_api::{HealthResponse, SystemStatus};
use renewal_services::system;

use crate::auth::CurrentUser;
use crate::dto::system_status;
use crate::error::ApiFailure;
use crate::state::AppState;
use crate::VERSION;

/// Unauthenticated. The client calls this first to decide between Setup, Bootstrap and Login.
pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let database = renewal_db::ping(&state.pool).await.is_ok();
    let users_exist = if database {
        system::users_exist(&state.pool).await.unwrap_or(false)
    } else {
        false
    };
    Json(HealthResponse {
        ok: database,
        version: VERSION.to_owned(),
        database,
        users_exist,
    })
}

pub async fn status(
    State(state): State<AppState>,
    CurrentUser(_caller): CurrentUser,
) -> Result<Json<SystemStatus>, ApiFailure> {
    let info = system::info(&state.pool).await?;
    Ok(Json(system_status(info)))
}
