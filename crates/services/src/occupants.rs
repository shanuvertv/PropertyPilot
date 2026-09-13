//! Occupants of a unit (shared accommodation): who lives there, since when, until when.

use chrono::NaiveDate;
use renewal_core::Capability;
use renewal_db::occupants::{self, OccupantInput, OccupantRow};
use renewal_db::{units, PgPool};
use uuid::Uuid;

use crate::audit_log::{self, NONE};
use crate::buildings::trim_opt;
use crate::error::{ServiceError, ServiceResult};
use crate::session::Session;

pub async fn for_unit(
    pool: &PgPool,
    caller: &Session,
    unit_id: Uuid,
    include_past: bool,
) -> ServiceResult<Vec<OccupantRow>> {
    caller.require(Capability::ViewOccupants)?;
    Ok(occupants::for_unit(pool, unit_id, include_past).await?)
}

pub async fn get(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<OccupantRow> {
    caller.require(Capability::ViewOccupants)?;
    occupants::find(pool, id)
        .await?
        .ok_or(ServiceError::NotFound("occupant"))
}

fn validate(input: &mut OccupantInput) -> ServiceResult<()> {
    input.full_name = input.full_name.trim().to_owned();
    trim_opt(&mut input.id_number);
    trim_opt(&mut input.phone);
    trim_opt(&mut input.email);
    trim_opt(&mut input.bed_label);
    trim_opt(&mut input.notes);
    if input.full_name.is_empty() {
        return Err(ServiceError::validation("the occupant's name is required"));
    }
    if let Some(out) = input.move_out {
        if out < input.move_in {
            return Err(ServiceError::validation(
                "move-out date cannot be before the move-in date",
            ));
        }
    }
    Ok(())
}

pub async fn create(
    pool: &PgPool,
    caller: &Session,
    unit_id: Uuid,
    mut input: OccupantInput,
) -> ServiceResult<OccupantRow> {
    caller.require(Capability::ManageOccupants)?;
    validate(&mut input)?;
    let unit = units::find(pool, unit_id)
        .await?
        .ok_or(ServiceError::NotFound("unit"))?;
    // Default the tenant to the company holding the unit's current contract.
    if input.tenant_id.is_none() {
        input.tenant_id = unit.tenant_id;
    }
    let mut tx = pool.begin().await?;
    let id = occupants::insert(&mut *tx, unit_id, &input, caller.user_id).await?;
    let row = occupants::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("occupant"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "occupant",
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
    mut input: OccupantInput,
) -> ServiceResult<OccupantRow> {
    caller.require(Capability::ManageOccupants)?;
    validate(&mut input)?;
    let mut tx = pool.begin().await?;
    let before = occupants::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("occupant"))?;
    occupants::update(&mut *tx, id, &input).await?;
    let after = occupants::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("occupant"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "occupant",
        id,
        "UPDATED",
        Some(&before),
        Some(&after),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

/// Sets (or clears) the move-out date. Occupants are never deleted once they have
/// expense shares; moving them out keeps the history intact.
pub async fn move_out(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    move_out: Option<NaiveDate>,
) -> ServiceResult<OccupantRow> {
    caller.require(Capability::ManageOccupants)?;
    let mut tx = pool.begin().await?;
    let before = occupants::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("occupant"))?;
    if let Some(d) = move_out {
        if d < before.move_in {
            return Err(ServiceError::validation(
                "move-out date cannot be before the move-in date",
            ));
        }
    }
    occupants::set_move_out(&mut *tx, id, move_out).await?;
    let after = occupants::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("occupant"))?;
    let action = if move_out.is_some() {
        "MOVED_OUT"
    } else {
        "REACTIVATED"
    };
    audit_log::log(
        &mut *tx,
        caller,
        "occupant",
        id,
        action,
        Some(&before),
        Some(&after),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

pub async fn delete(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<()> {
    caller.require(Capability::ManageOccupants)?;
    let mut tx = pool.begin().await?;
    let before = occupants::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("occupant"))?;
    if occupants::has_shares(&mut *tx, id).await? {
        return Err(ServiceError::Conflict(
            "this occupant has expense shares; record a move-out date instead of deleting".into(),
        ));
    }
    occupants::delete(&mut *tx, id).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "occupant",
        id,
        "DELETED",
        Some(&before),
        NONE,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
