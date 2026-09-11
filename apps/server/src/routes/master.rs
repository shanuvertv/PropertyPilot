//! Phase 1 routes: buildings, units, tenants.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use renewal_api::*;
use renewal_db::paging::ListQuery;
use renewal_db::units::UnitFilter;
use renewal_services::{buildings, documents, tenants, units};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::dto;
use crate::error::ApiFailure;
use crate::state::AppState;

pub fn list_query(p: &ListParams) -> ListQuery {
    ListQuery::new(
        p.q.clone(),
        p.page,
        p.page_size,
        p.sort.clone(),
        p.dir.as_deref(),
    )
}

// ---------------------------------------------------------------- buildings

pub async fn list_buildings(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(p): Query<ListParams>,
) -> Result<Json<Page<Building>>, ApiFailure> {
    let q = list_query(&p);
    let res = buildings::list(&state.pool, &caller, &q).await?;
    Ok(Json(dto::page(res, q.page, q.page_size, dto::building)))
}

pub async fn building_options(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<Vec<Building>>, ApiFailure> {
    Ok(Json(
        buildings::options(&state.pool, &caller)
            .await?
            .into_iter()
            .map(dto::building)
            .collect(),
    ))
}

pub async fn get_building(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<BuildingDetail>, ApiFailure> {
    let building = buildings::get(&state.pool, &caller, id).await?;
    let summary = buildings::summary(&state.pool, &caller, id).await?;
    let units = if caller.role.allows(renewal_core::Capability::ViewUnits) {
        units::for_building(&state.pool, &caller, id).await?
    } else {
        vec![]
    };
    let documents = documents::list(&state.pool, &caller, "building", id).await?;
    Ok(Json(BuildingDetail {
        building: dto::building(building),
        summary: dto::building_summary(summary),
        units: units.into_iter().map(dto::unit).collect(),
        documents: documents.into_iter().map(dto::document).collect(),
    }))
}

fn building_input(i: BuildingInput) -> renewal_db::buildings::BuildingInput {
    renewal_db::buildings::BuildingInput {
        name: i.name,
        code: i.code,
        location: i.location,
        building_type: i.building_type,
        notes: i.notes,
    }
}

pub async fn create_building(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Json(input): Json<BuildingInput>,
) -> Result<(StatusCode, Json<Building>), ApiFailure> {
    let row = buildings::create(&state.pool, &caller, building_input(input)).await?;
    Ok((StatusCode::CREATED, Json(dto::building(row))))
}

pub async fn update_building(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<BuildingInput>,
) -> Result<Json<Building>, ApiFailure> {
    let row = buildings::update(&state.pool, &caller, id, building_input(input)).await?;
    Ok(Json(dto::building(row)))
}

pub async fn archive_building(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiFailure> {
    buildings::archive(&state.pool, &caller, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------- units

pub async fn list_units(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(p): Query<UnitListParams>,
) -> Result<Json<Page<UnitSummary>>, ApiFailure> {
    let q = list_query(&p.list());
    let f = UnitFilter {
        building_id: dto::uuid_opt(&p.building_id, "building")?,
        status: p.status.map(|s| s.to_string()),
        band: p.band.map(|b| b.to_string()),
        renewal_status: p.renewal_status.map(|s| s.to_string()),
        tenant_id: dto::uuid_opt(&p.tenant_id, "tenant")?,
        expiring_soon: p.expiring_soon,
    };
    let res = units::list(&state.pool, &caller, &f, &q).await?;
    Ok(Json(dto::page(res, q.page, q.page_size, dto::unit)))
}

pub async fn get_unit(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<UnitSummary>, ApiFailure> {
    Ok(Json(dto::unit(units::get(&state.pool, &caller, id).await?)))
}

fn unit_input(i: UnitInput) -> Result<renewal_db::units::UnitInput, ApiFailure> {
    Ok(renewal_db::units::UnitInput {
        building_id: dto::uuid(&i.building_id, "building")?,
        unit_number: i.unit_number,
        floor: i.floor,
        unit_type: i.unit_type,
        status: i.status.to_string(),
        notes: i.notes,
    })
}

pub async fn create_unit(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Json(input): Json<UnitInput>,
) -> Result<(StatusCode, Json<UnitSummary>), ApiFailure> {
    let row = units::create(&state.pool, &caller, unit_input(input)?).await?;
    Ok((StatusCode::CREATED, Json(dto::unit(row))))
}

pub async fn update_unit(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UnitInput>,
) -> Result<Json<UnitSummary>, ApiFailure> {
    let row = units::update(&state.pool, &caller, id, unit_input(input)?).await?;
    Ok(Json(dto::unit(row)))
}

pub async fn set_unit_status(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SetUnitStatusRequest>,
) -> Result<Json<UnitSummary>, ApiFailure> {
    let row = units::set_status(&state.pool, &caller, id, req.status).await?;
    Ok(Json(dto::unit(row)))
}

pub async fn archive_unit(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiFailure> {
    units::archive(&state.pool, &caller, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------- tenants

pub async fn list_tenants(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(p): Query<ListParams>,
) -> Result<Json<Page<Tenant>>, ApiFailure> {
    let q = list_query(&p);
    let res = tenants::list(&state.pool, &caller, &q).await?;
    Ok(Json(dto::page(res, q.page, q.page_size, dto::tenant)))
}

pub async fn tenant_options(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<Vec<Tenant>>, ApiFailure> {
    Ok(Json(
        tenants::options(&state.pool, &caller)
            .await?
            .into_iter()
            .map(dto::tenant)
            .collect(),
    ))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantDetail {
    pub tenant: Tenant,
    pub contracts: Vec<Contract>,
    pub cases: Vec<RenewalCase>,
    pub documents: Vec<DocumentInfo>,
}

pub async fn get_tenant(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<TenantDetail>, ApiFailure> {
    let tenant = tenants::get(&state.pool, &caller, id).await?;
    let history = tenants::history(&state.pool, &caller, id).await?;
    let documents = documents::list(&state.pool, &caller, "tenant", id).await?;
    Ok(Json(TenantDetail {
        tenant: dto::tenant(tenant),
        contracts: history.contracts.into_iter().map(dto::contract).collect(),
        cases: history.cases.into_iter().map(dto::case).collect(),
        documents: documents.into_iter().map(dto::document).collect(),
    }))
}

fn tenant_input(i: TenantInput) -> renewal_db::tenants::TenantInput {
    renewal_db::tenants::TenantInput {
        name: i.name,
        contact_person: i.contact_person,
        mobile: i.mobile,
        email: i.email,
        alt_contact: i.alt_contact,
        address: i.address,
        notes: i.notes,
    }
}

pub async fn create_tenant(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Json(input): Json<TenantInput>,
) -> Result<(StatusCode, Json<Tenant>), ApiFailure> {
    let row = tenants::create(&state.pool, &caller, tenant_input(input)).await?;
    Ok((StatusCode::CREATED, Json(dto::tenant(row))))
}

pub async fn update_tenant(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<TenantInput>,
) -> Result<Json<Tenant>, ApiFailure> {
    let row = tenants::update(&state.pool, &caller, id, tenant_input(input)).await?;
    Ok(Json(dto::tenant(row)))
}

pub async fn archive_tenant(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiFailure> {
    tenants::archive(&state.pool, &caller, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
