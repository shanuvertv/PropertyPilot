//! Unit management and the Unit-Wise Summary (spec §3, §16).

use renewal_core::{Capability, UnitStatus, UnitType};
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
    if let Some(t) = &input.unit_type {
        input.unit_type = Some(
            t.parse::<UnitType>()
                .map_err(|_| {
                    ServiceError::validation("the unit type must be Residential or Commercial")
                })?
                .to_string(),
        );
    }
    Ok(())
}

/// How many units one bulk add may create at a time.
pub const MAX_BULK_UNITS: usize = 500;

pub struct BulkUnits {
    pub building_id: Uuid,
    /// Unit numbers to create, e.g. `101 … 110`; duplicates and existing numbers are skipped.
    pub numbers: Vec<String>,
    /// Floor for every unit, or per unit when the lengths match.
    pub floors: Vec<Option<String>>,
    pub unit_type: Option<String>,
    pub notes: Option<String>,
}

pub struct BulkResult {
    pub created: usize,
    pub skipped: Vec<String>,
}

/// Adds a whole floor or building of units in one go (see `BulkUnits`); numbers that
/// already exist in the building are skipped, never duplicated.
pub async fn create_bulk(
    pool: &PgPool,
    caller: &Session,
    mut input: BulkUnits,
) -> ServiceResult<BulkResult> {
    caller.require(Capability::ManageUnits)?;
    if let Some(t) = &input.unit_type {
        input.unit_type = Some(
            t.parse::<UnitType>()
                .map_err(|_| {
                    ServiceError::validation("the unit type must be Residential or Commercial")
                })?
                .to_string(),
        );
    }
    let mut tx = pool.begin().await?;
    buildings::find(&mut *tx, input.building_id)
        .await?
        .filter(|b| b.archived_at.is_none())
        .ok_or(ServiceError::NotFound("building"))?;
    let existing: Vec<String> = units::existing_numbers(&mut *tx, input.building_id)
        .await?
        .into_iter()
        .map(|n| n.trim().to_ascii_uppercase())
        .collect();

    let mut numbers: Vec<String> = Vec::new();
    let mut floors: Vec<Option<String>> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for (i, raw) in input.numbers.iter().enumerate() {
        let number = raw.trim().to_owned();
        if number.is_empty() {
            continue;
        }
        let key = number.to_ascii_uppercase();
        if existing.contains(&key) || seen.contains(&key) {
            skipped.push(number);
            continue;
        }
        seen.push(key);
        let floor = input
            .floors
            .get(i)
            .cloned()
            .flatten()
            .or_else(|| input.floors.first().cloned().flatten())
            .map(|f| f.trim().to_owned())
            .filter(|f| !f.is_empty());
        numbers.push(number);
        floors.push(floor);
    }
    if numbers.is_empty() {
        return Err(ServiceError::validation(if skipped.is_empty() {
            "no unit numbers were given"
        } else {
            "every one of those unit numbers already exists in this building"
        }));
    }
    if numbers.len() > MAX_BULK_UNITS {
        return Err(ServiceError::validation(format!(
            "at most {MAX_BULK_UNITS} units can be added at once"
        )));
    }
    let notes = input
        .notes
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty());
    let ids = units::insert_many(
        &mut *tx,
        input.building_id,
        &numbers,
        &floors,
        input.unit_type.as_deref(),
        &UnitStatus::Vacant.to_string(),
        notes,
        caller.user_id,
    )
    .await?;
    audit_log::log(
        &mut *tx,
        caller,
        "building",
        input.building_id,
        "UNITS_ADDED",
        NONE,
        Some(&serde_json::json!({ "created": ids.len(), "numbers": numbers, "skipped": skipped })),
    )
    .await?;
    tx.commit().await?;
    Ok(BulkResult {
        created: ids.len(),
        skipped,
    })
}

/// Removes a unit created by mistake. Only possible while nothing references it —
/// anything with contracts or expenses behind it is archived instead.
pub async fn delete(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<()> {
    caller.require(Capability::ManageUnits)?;
    let mut tx = pool.begin().await?;
    let before = units::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("unit"))?;
    if units::has_history(&mut *tx, id).await? {
        return Err(ServiceError::Conflict(
            "this unit has contracts or expenses on it — archive it instead of deleting".into(),
        ));
    }
    units::delete(&mut *tx, id).await?;
    audit_log::log(&mut *tx, caller, "unit", id, "DELETED", Some(&before), NONE).await?;
    tx.commit().await?;
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
