//! Rental contract management (spec §5, §14) and the unit-occupancy rules (PLAN.md §3.6).

use chrono::{Datelike, NaiveDate};
use renewal_core::{Capability, ContractStatus, RenewalStatus, UnitStatus};
use renewal_db::contracts::{self, ContractFilter, ContractInput, ContractRow, InsertContract};
use renewal_db::paging::{ListQuery, PageResult};
use renewal_db::{buildings, renewals, tenants, units, PgConnection, PgPool};
use uuid::Uuid;

use crate::audit_log::{self, NONE};
use crate::buildings::trim_opt;
use crate::error::{ServiceError, ServiceResult};
use crate::session::Session;

pub async fn list(
    pool: &PgPool,
    caller: &Session,
    f: &ContractFilter,
    q: &ListQuery,
) -> ServiceResult<PageResult<ContractRow>> {
    caller.require(Capability::ViewContracts)?;
    Ok(contracts::list(pool, f, q).await?)
}

pub async fn get(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<ContractRow> {
    caller.require(Capability::ViewContracts)?;
    contracts::find(pool, id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))
}

pub async fn chain(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<Vec<ContractRow>> {
    caller.require(Capability::ViewContracts)?;
    Ok(contracts::chain(pool, id).await?)
}

pub async fn suggest_number(pool: &PgPool, caller: &Session) -> ServiceResult<String> {
    caller.require(Capability::ManageContracts)?;
    Ok(contracts::suggest_number(pool).await?)
}

/// Contract duration in whole months, end date inclusive (1 Jan → 31 Dec = 12).
pub fn months_between(start: NaiveDate, end: NaiveDate) -> i32 {
    let next = end.succ_opt().unwrap_or(end);
    let months = (next.year() - start.year()) * 12 + (next.month() as i32 - start.month() as i32);
    let months = if next.day() < start.day() {
        months - 1
    } else {
        months
    };
    months.max(0)
}

async fn validate(
    conn: &mut PgConnection,
    input: &mut ContractInput,
    except: Option<Uuid>,
) -> ServiceResult<()> {
    input.contract_number = input.contract_number.trim().to_owned();
    trim_opt(&mut input.rent_terms);
    trim_opt(&mut input.notes);
    if input.contract_number.is_empty() {
        return Err(ServiceError::validation("contract number is required"));
    }
    if input.end_date < input.start_date {
        return Err(ServiceError::validation(
            "the end date must be on or after the start date",
        ));
    }
    if input.unit_ids.is_empty() {
        return Err(ServiceError::validation("select at least one unit"));
    }
    if contracts::number_exists(&mut *conn, &input.contract_number, except).await? {
        return Err(ServiceError::Conflict(format!(
            "contract number {} is already in use",
            input.contract_number
        )));
    }
    tenants::find(&mut *conn, input.tenant_id)
        .await?
        .filter(|t| t.archived_at.is_none())
        .ok_or(ServiceError::NotFound("tenant"))?;
    buildings::find(&mut *conn, input.building_id)
        .await?
        .filter(|b| b.archived_at.is_none())
        .ok_or(ServiceError::NotFound("building"))?;
    input.unit_ids.sort();
    input.unit_ids.dedup();
    for t in &input.unit_terms {
        if !input.unit_ids.contains(&t.unit_id) {
            return Err(ServiceError::validation(
                "occupants or rent were given for a unit that is not on the contract",
            ));
        }
        if !(0..=500).contains(&t.occupant_count) {
            return Err(ServiceError::validation(
                "the number of occupants must be between 0 and 500",
            ));
        }
        if t.rent_amount_minor.is_some_and(|r| r < 0) {
            return Err(ServiceError::validation("the rent cannot be negative"));
        }
    }
    let found = units::find_many(&mut *conn, &input.unit_ids).await?;
    if found.len() != input.unit_ids.len() {
        return Err(ServiceError::NotFound("one of the selected units"));
    }
    if let Some(u) = found.iter().find(|u| u.building_id != input.building_id) {
        return Err(ServiceError::validation(format!(
            "unit {} belongs to a different building",
            u.unit_number
        )));
    }
    Ok(())
}

/// Units must not be under another ACTIVE contract when this one goes live.
async fn ensure_units_free(
    conn: &mut PgConnection,
    unit_ids: &[Uuid],
    except: Option<Uuid>,
) -> ServiceResult<()> {
    let busy = units::occupied_by_other_contract(&mut *conn, unit_ids, except).await?;
    if !busy.is_empty() {
        let rows = units::find_many(&mut *conn, &busy).await?;
        let names: Vec<String> = rows
            .iter()
            .map(|u| format!("{} ({})", u.unit_number, u.building_name))
            .collect();
        return Err(ServiceError::Conflict(format!(
            "already under an active contract: {}",
            names.join(", ")
        )));
    }
    Ok(())
}

pub async fn create(
    pool: &PgPool,
    caller: &Session,
    mut input: ContractInput,
    activate: bool,
) -> ServiceResult<ContractRow> {
    caller.require(Capability::ManageContracts)?;
    let mut tx = pool.begin().await?;
    validate(&mut tx, &mut input, None).await?;
    let status = if activate {
        ContractStatus::Active
    } else {
        ContractStatus::Draft
    };
    if activate {
        ensure_units_free(&mut tx, &input.unit_ids, None).await?;
    }
    let id = contracts::insert(
        &mut tx,
        &InsertContract {
            input: &input,
            status: &status.to_string(),
            previous_contract_id: None,
            root_contract_id: None,
            renewal_sequence: 0,
            created_by: caller.user_id,
        },
    )
    .await?;
    if activate {
        units::set_status_many(
            &mut *tx,
            &input.unit_ids,
            &UnitStatus::Occupied.to_string(),
            None,
        )
        .await?;
    }
    let row = contracts::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "contract",
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
    mut input: ContractInput,
) -> ServiceResult<ContractRow> {
    caller.require(Capability::ManageContracts)?;
    let mut tx = pool.begin().await?;
    let before = contracts::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    let status: ContractStatus = before
        .status
        .parse()
        .map_err(|_| ServiceError::Internal("bad status".into()))?;
    if !matches!(status, ContractStatus::Draft | ContractStatus::Active) {
        return Err(ServiceError::Conflict(format!(
            "a {} contract cannot be edited",
            before.status
        )));
    }
    validate(&mut tx, &mut input, Some(id)).await?;
    if status == ContractStatus::Active {
        // Tenant and building are fixed once live; units may change but must be free.
        if input.tenant_id != before.tenant_id || input.building_id != before.building_id {
            return Err(ServiceError::Conflict(
                "tenant and building cannot change on an active contract".into(),
            ));
        }
        ensure_units_free(&mut tx, &input.unit_ids, Some(id)).await?;
        let removed: Vec<Uuid> = before
            .unit_ids
            .iter()
            .copied()
            .filter(|u| !input.unit_ids.contains(u))
            .collect();
        if !removed.is_empty() {
            units::set_status_many(
                &mut *tx,
                &removed,
                &UnitStatus::Vacant.to_string(),
                Some(&["OCCUPIED"]),
            )
            .await?;
        }
        units::set_status_many(
            &mut *tx,
            &input.unit_ids,
            &UnitStatus::Occupied.to_string(),
            None,
        )
        .await?;
    }
    contracts::update(&mut tx, id, &input).await?;
    let after = contracts::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "contract",
        id,
        "UPDATED",
        Some(&before),
        Some(&after),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

/// Draft → Active: units become Occupied.
pub async fn activate(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<ContractRow> {
    caller.require(Capability::ManageContracts)?;
    let mut tx = pool.begin().await?;
    let before = contracts::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    let status: ContractStatus = before
        .status
        .parse()
        .map_err(|_| ServiceError::Internal("bad status".into()))?;
    status.transition(ContractStatus::Active)?;
    ensure_units_free(&mut tx, &before.unit_ids, Some(id)).await?;
    contracts::set_status(&mut *tx, id, &ContractStatus::Active.to_string()).await?;
    units::set_status_many(
        &mut *tx,
        &before.unit_ids,
        &UnitStatus::Occupied.to_string(),
        None,
    )
    .await?;
    let after = contracts::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "contract",
        id,
        "ACTIVATED",
        Some(&before.status),
        Some(&after.status),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

/// Releases units when a contract stops being live: Occupied → Vacant unless under maintenance.
pub(crate) async fn release_units(conn: &mut PgConnection, unit_ids: &[Uuid]) -> ServiceResult<()> {
    units::set_status_many(
        &mut *conn,
        unit_ids,
        &UnitStatus::Vacant.to_string(),
        Some(&["OCCUPIED"]),
    )
    .await?;
    Ok(())
}

/// Active → Terminated (spec §5). Frees the units and closes any open renewal case.
pub async fn terminate(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    reason: Option<&str>,
) -> ServiceResult<ContractRow> {
    caller.require(Capability::ManageContracts)?;
    let mut tx = pool.begin().await?;
    let before = contracts::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    let status: ContractStatus = before
        .status
        .parse()
        .map_err(|_| ServiceError::Internal("bad status".into()))?;
    status.transition(ContractStatus::Terminated)?;
    contracts::set_status(&mut *tx, id, &ContractStatus::Terminated.to_string()).await?;
    release_units(&mut tx, &before.unit_ids).await?;
    if let Some(case) = renewals::open_for_contract(&mut *tx, id).await? {
        renewals::set_status(&mut *tx, case.id, &RenewalStatus::Closed.to_string()).await?;
        audit_log::log(
            &mut *tx,
            caller,
            "renewal_case",
            case.id,
            "CLOSED",
            Some(&case.status),
            Some(&"CLOSED"),
        )
        .await?;
    }
    let after = contracts::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "contract",
        id,
        "TERMINATED",
        Some(&before.status),
        Some(&serde_json::json!({ "status": after.status, "reason": reason })),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

pub async fn assign(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    employee: Option<Uuid>,
) -> ServiceResult<ContractRow> {
    caller.require(Capability::ManageContracts)?;
    let mut tx = pool.begin().await?;
    let before = contracts::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    contracts::set_assigned(&mut *tx, id, employee).await?;
    let after = contracts::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "contract",
        id,
        "ASSIGNED",
        Some(&before.assigned_employee_name),
        Some(&after.assigned_employee_name),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

/// Marks ACTIVE contracts past their end date as EXPIRED and frees their units.
/// Called by the scheduler (Phase 6) and available to Admin as "Run now".
pub async fn expire_overdue(pool: &PgPool, actor: Option<&Session>) -> ServiceResult<Vec<Uuid>> {
    let ids = contracts::active_past_end(pool).await?;
    for id in &ids {
        let mut tx = pool.begin().await?;
        let unit_ids = contracts::unit_ids(&mut *tx, *id).await?;
        contracts::set_status(&mut *tx, *id, &ContractStatus::Expired.to_string()).await?;
        release_units(&mut tx, &unit_ids).await?;
        if let Some(a) = actor {
            audit_log::log(
                &mut *tx,
                a,
                "contract",
                *id,
                "EXPIRED",
                Some(&"ACTIVE"),
                Some(&"EXPIRED"),
            )
            .await?;
        } else {
            renewal_db::audit::record(
                &mut *tx,
                &renewal_db::audit::AuditEntry {
                    actor_id: None,
                    entity_type: "contract",
                    entity_id: Some(*id),
                    action: "EXPIRED",
                    before: Some(serde_json::json!("ACTIVE")),
                    after: Some(serde_json::json!("EXPIRED")),
                },
            )
            .await?;
        }
        tx.commit().await?;
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn duration_in_months_counts_full_terms() {
        assert_eq!(months_between(d(2026, 1, 1), d(2026, 12, 31)), 12);
        assert_eq!(months_between(d(2026, 1, 1), d(2027, 1, 1)), 12);
        assert_eq!(months_between(d(2026, 3, 15), d(2026, 9, 14)), 6);
        assert_eq!(months_between(d(2026, 3, 15), d(2026, 3, 20)), 0);
        assert_eq!(months_between(d(2026, 1, 1), d(2028, 12, 31)), 36);
    }
}
