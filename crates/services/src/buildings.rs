//! Property / building management (spec §2).

use renewal_core::Capability;
use renewal_db::buildings::{self, BuildingInput, BuildingRow};
use renewal_db::dashboard::{self, BuildingSummary};
use renewal_db::paging::{ListQuery, PageResult};
use renewal_db::PgPool;
use uuid::Uuid;

use crate::audit_log::{self, NONE};
use crate::error::{ServiceError, ServiceResult};
use crate::session::Session;

pub async fn list(
    pool: &PgPool,
    caller: &Session,
    q: &ListQuery,
) -> ServiceResult<PageResult<BuildingRow>> {
    caller.require(Capability::ViewBuildings)?;
    Ok(buildings::list(pool, q).await?)
}

pub async fn options(pool: &PgPool, caller: &Session) -> ServiceResult<Vec<BuildingRow>> {
    caller.require(Capability::ViewBuildings)?;
    Ok(buildings::all_active(pool).await?)
}

pub async fn get(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<BuildingRow> {
    caller.require(Capability::ViewBuildings)?;
    buildings::find(pool, id)
        .await?
        .filter(|b| b.archived_at.is_none())
        .ok_or(ServiceError::NotFound("building"))
}

pub async fn summary(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<BuildingSummary> {
    caller.require(Capability::ViewBuildings)?;
    Ok(dashboard::building_summary(pool, id).await?)
}

fn validate(input: &mut BuildingInput) -> ServiceResult<()> {
    input.name = input.name.trim().to_owned();
    input.code = input.code.trim().to_owned();
    trim_opt(&mut input.location);
    trim_opt(&mut input.building_type);
    trim_opt(&mut input.notes);
    if input.name.is_empty() {
        return Err(ServiceError::validation("building name is required"));
    }
    if input.code.is_empty() {
        return Err(ServiceError::validation("property code is required"));
    }
    Ok(())
}

pub(crate) fn trim_opt(v: &mut Option<String>) {
    if let Some(s) = v {
        let t = s.trim();
        if t.is_empty() {
            *v = None;
        } else if t.len() != s.len() {
            *s = t.to_owned();
        }
    }
}

pub async fn create(
    pool: &PgPool,
    caller: &Session,
    mut input: BuildingInput,
) -> ServiceResult<BuildingRow> {
    caller.require(Capability::ManageBuildings)?;
    validate(&mut input)?;
    let mut tx = pool.begin().await?;
    if buildings::code_exists(&mut *tx, &input.code, None).await? {
        return Err(ServiceError::Conflict(format!(
            "property code {} is already in use",
            input.code
        )));
    }
    let id = buildings::insert(&mut *tx, &input, caller.user_id).await?;
    let row = buildings::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("building"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "building",
        id,
        "CREATED",
        NONE,
        Some(&row),
    )
    .await?;
    tx.commit().await?;
    Ok(row)
}

pub async fn update(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    mut input: BuildingInput,
) -> ServiceResult<BuildingRow> {
    caller.require(Capability::ManageBuildings)?;
    validate(&mut input)?;
    let mut tx = pool.begin().await?;
    let before = buildings::find(&mut *tx, id)
        .await?
        .filter(|b| b.archived_at.is_none())
        .ok_or(ServiceError::NotFound("building"))?;
    if buildings::code_exists(&mut *tx, &input.code, Some(id)).await? {
        return Err(ServiceError::Conflict(format!(
            "property code {} is already in use",
            input.code
        )));
    }
    buildings::update(&mut *tx, id, &input).await?;
    let after = buildings::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("building"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "building",
        id,
        "UPDATED",
        Some(&before),
        Some(&after),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

/// Archives a building and its units. Refused while any unit is under an active contract.
pub async fn archive(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<()> {
    caller.require(Capability::ManageBuildings)?;
    let mut tx = pool.begin().await?;
    let before = buildings::find(&mut *tx, id)
        .await?
        .filter(|b| b.archived_at.is_none())
        .ok_or(ServiceError::NotFound("building"))?;
    if before.occupied_units > 0 {
        return Err(ServiceError::Conflict(format!(
            "{} unit(s) are occupied under active contracts; end those contracts first",
            before.occupied_units
        )));
    }
    buildings::archive(&mut tx, id).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "building",
        id,
        "ARCHIVED",
        Some(&before),
        NONE,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
