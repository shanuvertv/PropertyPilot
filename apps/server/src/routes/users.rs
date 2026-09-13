use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use renewal_api::{CreateUserRequest, SetUserActiveRequest, UserSummary};
use renewal_services::users;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::dto::user_summary;
use crate::error::ApiFailure;
use crate::state::AppState;

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<Vec<UserSummary>>, ApiFailure> {
    let rows = users::list(&state.pool, &caller).await?;
    Ok(Json(rows.into_iter().map(user_summary).collect()))
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserSummary>), ApiFailure> {
    let row = users::create(
        &state.pool,
        &caller,
        users::NewUserInput {
            name: &req.name,
            email: &req.email,
            role: req.role,
            password: &req.password,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(user_summary(row))))
}

pub async fn set_active(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SetUserActiveRequest>,
) -> Result<Json<UserSummary>, ApiFailure> {
    let row = users::set_active(&state.pool, &caller, id, req.active).await?;
    Ok(Json(user_summary(row)))
}

pub async fn delete(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiFailure> {
    users::delete(&state.pool, &caller, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
