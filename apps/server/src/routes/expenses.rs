//! Expenses per unit with bill splitting, and the expenses dashboard.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use renewal_api::*;
use renewal_db::expenses::ExpenseFilter;
use renewal_services::expenses;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::dto;
use crate::error::ApiFailure;
use crate::routes::master::list_query;
use crate::state::{AppState, LiveEvent};

// ---------------------------------------------------------------- expenses

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseListQuery {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub sort: Option<String>,
    pub dir: Option<String>,
    pub unit_id: Option<String>,
    pub building_id: Option<String>,
    pub category: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub outstanding: Option<bool>,
}

pub async fn list_expenses(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(p): Query<ExpenseListQuery>,
) -> Result<Json<Page<Expense>>, ApiFailure> {
    let filter = ExpenseFilter {
        unit_id: dto::uuid_opt(&p.unit_id, "unit")?,
        building_id: dto::uuid_opt(&p.building_id, "building")?,
        category: p.category.clone().filter(|c| !c.is_empty()),
        from: dto::date_opt(&p.from, "from")?,
        to: dto::date_opt(&p.to, "to")?,
        outstanding: p.outstanding,
    };
    let lp = ListParams {
        q: p.q.clone(),
        page: p.page,
        page_size: p.page_size,
        sort: p.sort.clone().or_else(|| Some("expense_date".into())),
        dir: p.dir.clone().or_else(|| Some("desc".into())),
    };
    let q = list_query(&lp);
    let page = expenses::list(&state.pool, &caller, &filter, &q).await?;
    Ok(Json(dto::page(page, q.page, q.page_size, dto::expense)))
}

pub async fn create_expense(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Json(input): Json<ExpenseInput>,
) -> Result<(StatusCode, Json<Expense>), ApiFailure> {
    let row = expenses::create(&state.pool, &caller, dto::expense_input(&input)?).await?;
    state.publish(LiveEvent::data());
    Ok((StatusCode::CREATED, Json(dto::expense(row))))
}

pub async fn get_expense(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ExpenseDetail>, ApiFailure> {
    let detail = expenses::get(&state.pool, &caller, id).await?;
    Ok(Json(dto::expense_detail(detail)))
}

pub async fn update_expense(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<ExpenseInput>,
) -> Result<Json<Expense>, ApiFailure> {
    let row = expenses::update(&state.pool, &caller, id, dto::expense_input(&input)?).await?;
    state.publish(LiveEvent::data());
    Ok(Json(dto::expense(row)))
}

pub async fn delete_expense(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiFailure> {
    expenses::delete(&state.pool, &caller, id).await?;
    state.publish(LiveEvent::data());
    Ok(StatusCode::NO_CONTENT)
}

/// Split the bill evenly between N people (default: the unit's number of tenants).
pub async fn split_equal(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    req: Option<Json<SplitRequest>>,
) -> Result<Json<ExpenseDetail>, ApiFailure> {
    let count = match req.and_then(|Json(r)| r.split_count) {
        Some(n) => Some(dto::count(n, "number of people")?),
        None => None,
    };
    let detail = expenses::split_equal(&state.pool, &caller, id, count).await?;
    state.publish(LiveEvent::data());
    Ok(Json(dto::expense_detail(detail)))
}

/// Record how many people have paid their share.
pub async fn settle(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SettleRequest>,
) -> Result<Json<ExpenseDetail>, ApiFailure> {
    let n = dto::count(req.settled_count, "number paid")?;
    let detail = expenses::settle(&state.pool, &caller, id, n).await?;
    state.publish(LiveEvent::data());
    Ok(Json(dto::expense_detail(detail)))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryQuery {
    pub building_id: Option<String>,
    pub unit_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

pub async fn summary(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(q): Query<SummaryQuery>,
) -> Result<Json<ExpenseSummary>, ApiFailure> {
    let s = expenses::summary(
        &state.pool,
        &caller,
        dto::uuid_opt(&q.building_id, "building")?,
        dto::uuid_opt(&q.unit_id, "unit")?,
        dto::date_opt(&q.from, "from")?,
        dto::date_opt(&q.to, "to")?,
    )
    .await?;
    Ok(Json(dto::expense_summary(s)))
}
