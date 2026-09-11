//! `CurrentUser` extractor: resolves `Authorization: Bearer <token>` to a live session.

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use renewal_services::{auth, Session};

use crate::error::ApiFailure;
use crate::state::AppState;

pub struct CurrentUser(pub Session);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiFailure;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or_else(ApiFailure::unauthorized)?;
        match auth::authenticate(&state.pool, token).await? {
            Some(session) => Ok(CurrentUser(session)),
            None => Err(ApiFailure::unauthorized()),
        }
    }
}

pub fn user_agent(parts: &axum::http::HeaderMap) -> Option<String> {
    parts
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.chars().take(200).collect())
}
