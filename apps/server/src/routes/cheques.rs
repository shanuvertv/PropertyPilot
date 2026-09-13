//! Rent cheques per contract, the cross-contract cheque list and its summary.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use renewal_api::*;
use renewal_core::ChequeStatus;
use renewal_db::cheques::ChequeFilter;
use renewal_services::cheques;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::dto;
use crate::error::ApiFailure;
use crate::routes::master::list_query;
use crate::state::{AppState, LiveEvent};

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(p): Query<ChequeListParams>,
) -> Result<Json<Page<Cheque>>, ApiFailure> {
    let filter = ChequeFilter {
        contract_id: dto::uuid_opt(&p.contract_id, "contract")?,
        building_id: dto::uuid_opt(&p.building_id, "building")?,
        tenant_id: dto::uuid_opt(&p.tenant_id, "tenant")?,
        status: p.status.as_deref().filter(|s| !s.is_empty()).map(|s| {
            s.split(',')
                .map(|x| x.trim().to_ascii_uppercase())
                .filter(|x| !x.is_empty())
                .collect()
        }),
        due_from: dto::date_opt(&p.due_from, "from")?,
        due_to: dto::date_opt(&p.due_to, "to")?,
        overdue: p.overdue,
    };
    let lp = ListParams {
        q: p.q.clone(),
        page: p.page,
        page_size: p.page_size,
        sort: p.sort.clone().or_else(|| Some("due_date".into())),
        dir: p.dir.clone(),
    };
    let q = list_query(&lp);
    let page = cheques::list(&state.pool, &caller, &filter, &q).await?;
    Ok(Json(dto::page(page, q.page, q.page_size, dto::cheque)))
}

pub async fn summary(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<ChequeSummary>, ApiFailure> {
    let s = cheques::summary(&state.pool, &caller).await?;
    let reminder_days = cheques::reminder_days(&state.pool).await?;
    Ok(Json(dto::cheque_summary(s, reminder_days)))
}

pub async fn for_contract(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(contract_id): Path<Uuid>,
) -> Result<Json<Vec<Cheque>>, ApiFailure> {
    let rows = cheques::for_contract(&state.pool, &caller, contract_id).await?;
    Ok(Json(rows.into_iter().map(dto::cheque).collect()))
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(contract_id): Path<Uuid>,
    Json(input): Json<ChequeInput>,
) -> Result<(StatusCode, Json<Cheque>), ApiFailure> {
    let row = cheques::create(
        &state.pool,
        &caller,
        contract_id,
        dto::cheque_input(&input)?,
    )
    .await?;
    state.publish(LiveEvent::data());
    Ok((StatusCode::CREATED, Json(dto::cheque(row))))
}

pub async fn generate(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<GenerateChequesRequest>,
) -> Result<Json<Vec<Cheque>>, ApiFailure> {
    let input = cheques::GenerateInput {
        count: usize::try_from(req.count).unwrap_or(0),
        first_date: dto::date_opt(&req.first_date, "first cheque date")?,
        every_months: match req.every_months {
            Some(m) => Some(u32::try_from(m).map_err(|_| {
                ApiFailure(renewal_services::ServiceError::validation(
                    "months between cheques must be 0 or more",
                ))
            })?),
            None => None,
        },
        total_minor: req.total.map(|t| dto::minor(t, "total")).transpose()?,
        bank_name: req.bank_name,
        replace_pending: req.replace_pending,
    };
    let rows = cheques::generate(&state.pool, &caller, contract_id, input).await?;
    state.publish(LiveEvent::data());
    Ok(Json(rows.into_iter().map(dto::cheque).collect()))
}

pub async fn get(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Cheque>, ApiFailure> {
    Ok(Json(dto::cheque(
        cheques::get(&state.pool, &caller, id).await?,
    )))
}

pub async fn update(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<ChequeInput>,
) -> Result<Json<Cheque>, ApiFailure> {
    let row = cheques::update(&state.pool, &caller, id, dto::cheque_input(&input)?).await?;
    state.publish(LiveEvent::data());
    Ok(Json(dto::cheque(row)))
}

pub async fn set_status(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ChequeStatusRequest>,
) -> Result<Json<Cheque>, ApiFailure> {
    let status: ChequeStatus = req.status.parse().map_err(|_| {
        ApiFailure(renewal_services::ServiceError::validation(
            "invalid cheque status",
        ))
    })?;
    let row = cheques::set_status(&state.pool, &caller, id, status).await?;
    state.publish(LiveEvent::data());
    Ok(Json(dto::cheque(row)))
}

pub async fn delete(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiFailure> {
    cheques::delete(&state.pool, &caller, id).await?;
    state.publish(LiveEvent::data());
    Ok(StatusCode::NO_CONTENT)
}
