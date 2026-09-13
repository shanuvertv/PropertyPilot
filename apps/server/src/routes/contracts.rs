//! Phase 2 routes: contracts and the contract timeline.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use renewal_api::*;
use renewal_db::contracts::ContractFilter;
use renewal_services::{contracts, documents, renewals};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::dto;
use crate::error::ApiFailure;
use crate::routes::master::list_query;
use crate::state::AppState;

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(p): Query<ContractListParams>,
) -> Result<Json<Page<Contract>>, ApiFailure> {
    let q = list_query(&p.list());
    let f = ContractFilter {
        status: p.status.map(|s| vec![s.to_string()]),
        band: p.band.map(|b| b.to_string()),
        tenant_id: dto::uuid_opt(&p.tenant_id, "tenant")?,
        building_id: dto::uuid_opt(&p.building_id, "building")?,
        expiring_soon: p.expiring_soon,
        urgent: p.urgent,
        has_open_case: p.has_open_case,
        renewal_status: p.renewal_status.map(|s| vec![s.to_string()]),
        ..Default::default()
    };
    let res = contracts::list(&state.pool, &caller, &f, &q).await?;
    Ok(Json(dto::page(res, q.page, q.page_size, dto::contract)))
}

pub async fn suggest_number(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<NumberSuggestion>, ApiFailure> {
    Ok(Json(NumberSuggestion {
        contract_number: contracts::suggest_number(&state.pool, &caller).await?,
    }))
}

pub async fn get(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ContractDetail>, ApiFailure> {
    let row = contracts::get(&state.pool, &caller, id).await?;
    let unit_rows = if caller.role.allows(renewal_core::Capability::ViewUnits) {
        renewal_db::units::find_many(&state.pool, &row.unit_ids)
            .await
            .map_err(renewal_services::ServiceError::Db)?
    } else {
        vec![]
    };
    let chain = contracts::chain(&state.pool, &caller, id).await?;
    let documents = documents::list(&state.pool, &caller, "contract", id).await?;
    let open_case = if caller.role.allows(renewal_core::Capability::ViewRenewals) {
        match row.case_id {
            Some(cid) => Some(renewals::get(&state.pool, &caller, cid).await?),
            None => None,
        }
    } else {
        None
    };
    Ok(Json(ContractDetail {
        contract: dto::contract(row),
        units: unit_rows.into_iter().map(dto::unit).collect(),
        chain: chain.into_iter().map(dto::contract).collect(),
        documents: documents.into_iter().map(dto::document).collect(),
        open_case: open_case.map(dto::case),
    }))
}

fn contract_input(i: &ContractInput) -> Result<renewal_db::contracts::ContractInput, ApiFailure> {
    Ok(renewal_db::contracts::ContractInput {
        contract_number: i.contract_number.clone(),
        tenant_id: dto::uuid(&i.tenant_id, "tenant")?,
        building_id: dto::uuid(&i.building_id, "building")?,
        unit_ids: i
            .unit_ids
            .iter()
            .map(|u| dto::uuid(u, "unit"))
            .collect::<Result<Vec<_>, _>>()?,
        unit_tenants: i
            .unit_tenants
            .iter()
            .map(|t| {
                Ok((
                    dto::uuid(&t.unit_id, "unit")?,
                    dto::count(t.occupant_count, "number of tenants")?,
                ))
            })
            .collect::<Result<Vec<_>, ApiFailure>>()?,
        start_date: dto::date(&i.start_date, "start date")?,
        end_date: dto::date(&i.end_date, "end date")?,
        rent_terms: i.rent_terms.clone(),
        rent_amount_minor: i
            .rent_amount
            .map(|a| dto::minor(a, "rent amount"))
            .transpose()?,
        assigned_employee_id: dto::uuid_opt(&i.assigned_employee_id, "assigned employee")?,
        notes: i.notes.clone(),
    })
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Json(input): Json<ContractInput>,
) -> Result<(StatusCode, Json<Contract>), ApiFailure> {
    let activate = input.activate.unwrap_or(true);
    let row = contracts::create(&state.pool, &caller, contract_input(&input)?, activate).await?;
    Ok((StatusCode::CREATED, Json(dto::contract(row))))
}

pub async fn update(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<ContractInput>,
) -> Result<Json<Contract>, ApiFailure> {
    let row = contracts::update(&state.pool, &caller, id, contract_input(&input)?).await?;
    Ok(Json(dto::contract(row)))
}

pub async fn activate(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Contract>, ApiFailure> {
    Ok(Json(dto::contract(
        contracts::activate(&state.pool, &caller, id).await?,
    )))
}

pub async fn terminate(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<TerminateContractRequest>,
) -> Result<Json<Contract>, ApiFailure> {
    Ok(Json(dto::contract(
        contracts::terminate(&state.pool, &caller, id, req.reason.as_deref()).await?,
    )))
}

pub async fn assign(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AssignRequest>,
) -> Result<Json<Contract>, ApiFailure> {
    let emp = dto::uuid_opt(&req.assigned_employee_id, "assigned employee")?;
    Ok(Json(dto::contract(
        contracts::assign(&state.pool, &caller, id, emp).await?,
    )))
}

/// Spec §7 step 2: open a renewal case on this contract.
pub async fn start_renewal(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<StartRenewalRequest>,
) -> Result<(StatusCode, Json<RenewalCase>), ApiFailure> {
    let emp = dto::uuid_opt(&req.assigned_employee_id, "assigned employee")?;
    let row = renewals::start(&state.pool, &caller, id, emp).await?;
    Ok((StatusCode::CREATED, Json(dto::case(row))))
}

/// Admin "Run now": mark overdue ACTIVE contracts as Expired (the scheduler does this nightly from Phase 6).
pub async fn expire_overdue(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<Vec<String>>, ApiFailure> {
    caller.require(renewal_core::Capability::ManageSettings)?;
    let ids = contracts::expire_overdue(&state.pool, Some(&caller)).await?;
    Ok(Json(ids.into_iter().map(|i| i.to_string()).collect()))
}
