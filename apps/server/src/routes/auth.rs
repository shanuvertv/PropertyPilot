use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use renewal_api::{
    BootstrapRequest, ChangePasswordRequest, LoginRequest, LoginResponse, ResetPasswordRequest,
    SessionInfo,
};
use renewal_services::{auth, users};
use std::net::SocketAddr;
use uuid::Uuid;

use crate::auth::{user_agent, CurrentUser};
use crate::dto::session_info;
use crate::error::{ApiFailure, LoginFailure};
use crate::state::AppState;

pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, LoginFailure> {
    // Rate limit per (client, mailbox) so one guessed account cannot lock the whole office out.
    let key = format!("{}|{}", peer.ip(), req.email.trim().to_ascii_lowercase());
    if !state.login_limiter.allow(&key) {
        return Err(LoginFailure::RateLimited);
    }
    let issued = auth::login(
        &state.pool,
        &req.email,
        &req.password,
        user_agent(&headers).as_deref(),
    )
    .await?;
    state.login_limiter.reset(&key);
    Ok(Json(LoginResponse {
        token: issued.token,
        session: session_info(&issued.session),
    }))
}

pub async fn bootstrap(
    State(state): State<AppState>,
    Json(req): Json<BootstrapRequest>,
) -> Result<(StatusCode, Json<LoginResponse>), ApiFailure> {
    let issued = auth::bootstrap_admin(&state.pool, &req.name, &req.email, &req.password).await?;
    Ok((
        StatusCode::CREATED,
        Json(LoginResponse {
            token: issued.token,
            session: session_info(&issued.session),
        }),
    ))
}

pub async fn logout(
    State(state): State<AppState>,
    CurrentUser(session): CurrentUser,
) -> Result<StatusCode, ApiFailure> {
    auth::logout(&state.pool, &session).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn me(CurrentUser(session): CurrentUser) -> Json<SessionInfo> {
    Json(session_info(&session))
}

/// The signed-in user changes their own password; every other session they hold is revoked.
pub async fn change_password(
    State(state): State<AppState>,
    CurrentUser(session): CurrentUser,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<StatusCode, ApiFailure> {
    users::change_password(
        &state.pool,
        &session,
        &req.current_password,
        &req.new_password,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Admin sets a temporary password for a user; that user is signed out everywhere.
pub async fn reset_password(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<StatusCode, ApiFailure> {
    users::reset_password(&state.pool, &caller, id, &req.new_password).await?;
    Ok(StatusCode::NO_CONTENT)
}
