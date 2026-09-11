//! Phase 3 routes: main dashboard, global search, employee options.

use axum::extract::{Query, State};
use axum::Json;
use renewal_api::{Dashboard, EmployeeOption, SearchHit};
use renewal_services::dashboard;

use crate::auth::CurrentUser;
use crate::dto;
use crate::error::ApiFailure;
use crate::state::AppState;

pub async fn get(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<Dashboard>, ApiFailure> {
    Ok(Json(dto::dashboard(
        dashboard::load(&state.pool, &caller).await?,
    )))
}

#[derive(serde::Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

pub async fn search(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(p): Query<SearchQuery>,
) -> Result<Json<Vec<SearchHit>>, ApiFailure> {
    let hits = dashboard::search(&state.pool, &caller, p.q.as_deref().unwrap_or("")).await?;
    Ok(Json(hits.into_iter().filter_map(dto::search_hit).collect()))
}

pub async fn employees(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<Vec<EmployeeOption>>, ApiFailure> {
    Ok(Json(
        dashboard::employees(&state.pool, &caller)
            .await?
            .into_iter()
            .map(dto::employee)
            .collect(),
    ))
}
