//! Unit management and the Unit-Wise Summary (spec §3, §16).

use renewal_core::{Capability, UnitStatus};
use renewal_db::paging::{ListQuery, PageResult};
use renewal_db::units::{self, UnitFilter, UnitInput, UnitSummaryRow};
use renewal_db::{buildings, PgPool};
use uuid::Uuid;

use crate::audit_log::{self, NONE};
use crate::buildings::trim_opt;
use crate::error::{ServiceError, ServiceResult};
use crate::session::Session;

pub async fn list(
    pool: &PgPool,
    caller: &Session,
    f: &UnitFilter,
    q: &ListQuery,
) -> ServiceResult<PageResult<UnitSummaryRow>> {
    caller.require(Capability::ViewUnits)?;
    Ok(units::list(pool, f, q).await?)
}

pub async fn get(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<UnitSummaryRow> {
    caller.require(Capability::ViewUnits)?;
    units::find(pool, id)
        .await?
        .ok_or(ServiceError::NotFound("unit"))
}

pub async fn for_building(
    pool: &PgPool,
    caller: &Session,
    building_id: Uuid,
) -> ServiceResult<Vec<UnitSummaryRow>> {
    caller.require(Capability::ViewUnits)?;
    Ok(units::for_building(pool, building_id).await?)
}

fn validate(input: &mut UnitInput) -> ServiceResult<()> {
    input.unit_number = input.unit_number.trim().to_owned();
    trim_opt(&mut input.floor);
    trim_opt(&mut input.unit_type);
    trim_opt(&mut input.notes);
    if input.unit_number.is_empty() {
        return Err(ServiceError::validation("unit number is required"));
    }
    input
        .status
        .parse::<UnitStatus>()
        .map_err(|_| ServiceError::validation("invalid unit status"))?;
    Ok(())
}

pub async fn create(
    pool: &PgPool,
    caller: &Session,
    mut input: UnitInput,
) -> ServiceResult<UnitSummaryRow> {
    caller.require(Capability::ManageUnits)?;
    validate(&mut input)?;
    if input.status == UnitStatus::Occupied.to_string() {
        return Err(ServiceError::validation(
            "a unit becomes Occupied through an active contract, not manually",
        ));
    }
    let mut tx = pool.begin().await?;
    buildings::find(&mut *tx, input.building_id)
        .await?
        .filter(|b| b.archived_at.is_none())
        .ok_or(ServiceError::NotFound("building"))?;
    if units::number_exists(&mut *tx, input.building_id, &input.unit_number, None).await? {
        return Err(ServiceError::Conflict(format!(
            "unit {} already exists in this building",
            input.unit_number
        )));
    }
    let id = units::insert(&mut *tx, &input, caller.user_id).await?;
    let row = units::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("unit"))?;
    audit_log::log(&mut *tx, caller, "unit", id, "CREATED", NONE, Some(&row)).await?;
    tx.commit().await?;
    Ok(row)
}

pub async fn update(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    mut input: UnitInput,
) -> ServiceResult<UnitSummaryRow> {
    caller.require(Capability::ManageUnits)?;
    validate(&mut input)?;
    let mut tx = pool.begin().await?;
    let before = units::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("unit"))?;
    // Occupancy is owned by contracts: keep whatever the contract engine set.
    if before.contract_id.is_some() {
        input.status = before.status.clone();
        if input.building_id != before.building_id {
            return Err(ServiceError::Conflict(
                "a unit under an active contract cannot move to another building".into(),
            ));
        }
    } else if input.status == UnitStatus::Occupied.to_string() {
        return Err(ServiceError::validation(
            "a unit becomes Occupied through an active contract, not manually",
        ));
    }
    if units::number_exists(&mut *tx, input.building_id, &input.unit_number, Some(id)).await? {
        return Err(ServiceError::Conflict(format!(
            "unit {} already exists in this building",
            input.unit_number
        )));
    }
    units::update(&mut *tx, id, &input).await?;
    let after = units::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("unit"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "unit",
        id,
        "UPDATED",
        Some(&before),
        Some(&after),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

/// Operations may flip Vacant ⇄ Reserved ⇄ Under Maintenance; Occupied is contract-driven.
pub async fn set_status(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    status: UnitStatus,
) -> ServiceResult<UnitSummaryRow> {
    caller.require(Capability::UpdateUnitStatus)?;
    let mut tx = pool.begin().await?;
    let before = units::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("unit"))?;
    if before.contract_id.is_some() {
        return Err(ServiceError::Conflict(
            "this unit is under an active contract; its status follows the contract".into(),
        ));
    }
    if status == UnitStatus::Occupied {
        return Err(ServiceError::validation(
            "a unit becomes Occupied through an active contract, not manually",
        ));
    }
    units::set_status(&mut *tx, id, &status.to_string()).await?;
    let after = units::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("unit"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "unit",
        id,
        "STATUS_CHANGED",
        Some(&before.status),
        Some(&after.status),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

pub async fn archive(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<()> {
    caller.require(Capability::ManageUnits)?;
    let mut tx = pool.begin().await?;
    let before = units::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("unit"))?;
    if before.contract_id.is_some() {
        return Err(ServiceError::Conflict(
            "this unit is under an active contract and cannot be archived".into(),
        ));
    }
    units::archive(&mut *tx, id).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "unit",
        id,
        "ARCHIVED",
        Some(&before),
        NONE,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
