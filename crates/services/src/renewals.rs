//! Renewal workflow (spec §6, §7, §11, §13, §14; PLAN.md §6.2–6.4).

use chrono::NaiveDate;
use renewal_core::{Capability, ContractStatus, RenewalStatus, TenantResponse, UnitStatus};
use renewal_db::contracts::{self, ContractInput, ContractRow, InsertContract};
use renewal_db::paging::{ListQuery, PageResult};
use renewal_db::renewals::{
    self, CaseFilter, CaseRow, ChecklistItemRow, ResponseRow, TemplateItemInput, TemplateItemRow,
};
use renewal_db::{follow_ups, units, PgConnection, PgPool};
use strum::IntoEnumIterator;
use uuid::Uuid;

use crate::audit_log::{self, NONE};
use crate::buildings::trim_opt;
use crate::error::{ServiceError, ServiceResult};
use crate::session::Session;

pub async fn list(
    pool: &PgPool,
    caller: &Session,
    f: &CaseFilter,
    q: &ListQuery,
) -> ServiceResult<PageResult<CaseRow>> {
    caller.require(Capability::ViewRenewals)?;
    Ok(renewals::list(pool, f, q).await?)
}

pub async fn get(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<CaseRow> {
    caller.require(Capability::ViewRenewals)?;
    renewals::find(pool, id)
        .await?
        .ok_or(ServiceError::NotFound("renewal case"))
}

pub async fn responses(
    pool: &PgPool,
    caller: &Session,
    case_id: Uuid,
) -> ServiceResult<Vec<ResponseRow>> {
    caller.require(Capability::ViewRenewals)?;
    Ok(renewals::responses(pool, case_id).await?)
}

pub async fn checklist(
    pool: &PgPool,
    caller: &Session,
    case_id: Uuid,
) -> ServiceResult<Vec<ChecklistItemRow>> {
    caller.require(Capability::ViewRenewals)?;
    Ok(renewals::checklist(pool, case_id).await?)
}

fn parse_status(s: &str) -> ServiceResult<RenewalStatus> {
    s.parse()
        .map_err(|_| ServiceError::Internal(format!("bad renewal status {s}")))
}

pub fn allowed_transitions(from: RenewalStatus) -> Vec<RenewalStatus> {
    RenewalStatus::iter()
        .filter(|to| from.can_transition_to(*to))
        .collect()
}

/// Spec §7 step 2: the assigned employee opens a Renewal Case on a live contract.
pub async fn start(
    pool: &PgPool,
    caller: &Session,
    contract_id: Uuid,
    assigned: Option<Uuid>,
) -> ServiceResult<CaseRow> {
    caller.require(Capability::ManageRenewals)?;
    let mut tx = pool.begin().await?;
    let contract = contracts::find(&mut *tx, contract_id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    if contract.status != ContractStatus::Active.to_string() {
        return Err(ServiceError::Conflict(format!(
            "only active contracts can be renewed (this one is {})",
            contract.status
        )));
    }
    if let Some(existing) = renewals::open_for_contract(&mut *tx, contract_id).await? {
        return Err(ServiceError::Conflict(format!(
            "a renewal case is already open (status {})",
            existing.status
        )));
    }
    let assigned = assigned
        .or(contract.assigned_employee_id)
        .or(Some(caller.user_id));
    let id = renewals::insert(
        &mut tx,
        contract_id,
        assigned,
        contract.urgent,
        caller.user_id,
    )
    .await?;
    let row = renewals::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("renewal case"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "renewal_case",
        id,
        "RENEWAL_STARTED",
        NONE,
        Some(&row),
    )
    .await?;
    audit_log::log(
        &mut *tx,
        caller,
        "contract",
        contract_id,
        "RENEWAL_STARTED",
        NONE,
        Some(&serde_json::json!({ "caseId": id })),
    )
    .await?;
    tx.commit().await?;
    Ok(row)
}

async fn load_open(conn: &mut PgConnection, id: Uuid) -> ServiceResult<(CaseRow, RenewalStatus)> {
    let row = renewals::find(&mut *conn, id)
        .await?
        .ok_or(ServiceError::NotFound("renewal case"))?;
    let status = parse_status(&row.status)?;
    Ok((row, status))
}

pub async fn set_status(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    to: RenewalStatus,
) -> ServiceResult<CaseRow> {
    caller.require(Capability::ManageRenewals)?;
    if to == RenewalStatus::RenewalCompleted {
        return Err(ServiceError::validation(
            "use Complete renewal to finish a case; it creates the new contract",
        ));
    }
    let mut tx = pool.begin().await?;
    let (before, from) = load_open(&mut tx, id).await?;
    from.transition(to)?;
    renewals::set_status(&mut *tx, id, &to.to_string()).await?;
    if to == RenewalStatus::RenewalConfirmed {
        renewals::tick_by_key(&mut *tx, id, "CONFIRMED", caller.user_id).await?;
    }
    let after = renewals::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("renewal case"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "renewal_case",
        id,
        "STATUS_CHANGED",
        Some(&before.status),
        Some(&after.status),
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
) -> ServiceResult<CaseRow> {
    caller.require(Capability::ManageRenewals)?;
    let mut tx = pool.begin().await?;
    let (before, _) = load_open(&mut tx, id).await?;
    renewals::set_assigned(&mut *tx, id, employee).await?;
    let after = renewals::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("renewal case"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "renewal_case",
        id,
        "ASSIGNED",
        Some(&before.assigned_employee_name),
        Some(&after.assigned_employee_name),
    )
    .await?;
    tx.commit().await?;
    if let Some(assignee) = employee.filter(|e| *e != caller.user_id) {
        let title = format!(
            "{} assigned you the renewal of {} ({})",
            caller.name, after.tenant_name, after.contract_number
        );
        crate::notifications::case_assigned(pool, assignee, id, &title).await?;
    }
    Ok(after)
}

pub async fn set_notes(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    mut notes: Option<String>,
) -> ServiceResult<CaseRow> {
    caller.require(Capability::ManageRenewals)?;
    trim_opt(&mut notes);
    let mut tx = pool.begin().await?;
    let (before, _) = load_open(&mut tx, id).await?;
    renewals::set_notes(&mut *tx, id, notes.as_deref()).await?;
    let after = renewals::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("renewal case"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "renewal_case",
        id,
        "NOTES_UPDATED",
        Some(&before.notes),
        Some(&after.notes),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

pub struct ResponseInput {
    pub response: TenantResponse,
    pub response_date: NaiveDate,
    pub notes: Option<String>,
    pub follow_up_date: Option<NaiveDate>,
    pub follow_up_type: Option<String>,
}

/// Spec §11: record the tenant's answer. The case status follows the response
/// (PLAN.md §3.3), the checklist item "Tenant Response Received" is ticked, and an
/// optional follow-up is created.
pub async fn record_response(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    mut input: ResponseInput,
) -> ServiceResult<CaseRow> {
    caller.require(Capability::ManageRenewals)?;
    trim_opt(&mut input.notes);
    let mut tx = pool.begin().await?;
    let (before, from) = load_open(&mut tx, id).await?;
    if !from.is_open() {
        return Err(ServiceError::Conflict("this renewal case is closed".into()));
    }
    renewals::insert_response(
        &mut tx,
        id,
        &input.response.to_string(),
        input.response_date,
        input.notes.as_deref(),
        input.follow_up_date,
        caller.user_id,
    )
    .await?;
    if input.response != TenantResponse::NoResponse {
        renewals::tick_by_key(&mut *tx, id, "RESPONSE_RECEIVED", caller.user_id).await?;
    }
    let implied = input.response.implied_renewal_status();
    if implied != from && from.can_transition_to(implied) {
        renewals::set_status(&mut *tx, id, &implied.to_string()).await?;
        if implied == RenewalStatus::RenewalConfirmed {
            renewals::tick_by_key(&mut *tx, id, "CONFIRMED", caller.user_id).await?;
        }
    }
    if let Some(due) = input.follow_up_date {
        let kind = input
            .follow_up_type
            .clone()
            .unwrap_or_else(|| "PHONE_CALL".into());
        follow_ups::insert(
            &mut *tx,
            &follow_ups::NewFollowUp {
                case_id: id,
                due_date: due,
                follow_up_type: &kind,
                assigned_employee_id: before.assigned_employee_id.or(Some(caller.user_id)),
                notes: Some("Follow up on tenant response"),
                created_by: caller.user_id,
            },
        )
        .await?;
        renewals::set_next_follow_up(&mut *tx, id).await?;
    }
    let after = renewals::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("renewal case"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "renewal_case",
        id,
        "TENANT_RESPONSE_RECORDED",
        Some(&serde_json::json!({ "status": before.status, "latestResponse": before.latest_response })),
        Some(&serde_json::json!({ "status": after.status, "latestResponse": after.latest_response, "notes": input.notes })),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

pub async fn set_checklist_item(
    pool: &PgPool,
    caller: &Session,
    case_id: Uuid,
    item_id: Uuid,
    done: bool,
) -> ServiceResult<Vec<ChecklistItemRow>> {
    caller.require(Capability::ManageRenewals)?;
    let mut tx = pool.begin().await?;
    load_open(&mut tx, case_id).await?;
    let item = renewals::set_checklist_item(&mut *tx, item_id, done, caller.user_id)
        .await?
        .filter(|i| i.case_id == case_id)
        .ok_or(ServiceError::NotFound("checklist item"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "renewal_case",
        case_id,
        if done {
            "CHECKLIST_DONE"
        } else {
            "CHECKLIST_UNDONE"
        },
        NONE,
        Some(&item.label),
    )
    .await?;
    let list = renewals::checklist(&mut *tx, case_id).await?;
    tx.commit().await?;
    Ok(list)
}

pub struct CompletionInput {
    pub contract_number: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub rent_terms: Option<String>,
    /// New rent; `None` keeps the old contract's amount.
    pub rent_amount_minor: Option<i64>,
    pub notes: Option<String>,
}

/// Spec §14 / PLAN.md §6.4 — one transaction:
/// new linked contract (Active) → old contract Renewed → units stay Occupied → case Renewal Completed.
pub async fn complete(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    mut input: CompletionInput,
) -> ServiceResult<(CaseRow, ContractRow)> {
    caller.require(Capability::ManageRenewals)?;
    caller.require(Capability::ManageContracts)?;
    trim_opt(&mut input.rent_terms);
    trim_opt(&mut input.notes);
    if input.end_date < input.start_date {
        return Err(ServiceError::validation(
            "the new end date must be on or after the new start date",
        ));
    }
    let mut tx = pool.begin().await?;
    let (case, from) = load_open(&mut tx, id).await?;
    if from != RenewalStatus::RenewalConfirmed {
        return Err(ServiceError::Conflict(format!(
            "the case must be Renewal Confirmed before completion (it is {from})"
        )));
    }
    let open = renewals::required_open_items(&mut *tx, id).await?;
    if !open.is_empty() {
        return Err(ServiceError::Conflict(format!(
            "required checklist items are still open: {}",
            open.join(", ")
        )));
    }
    let old = contracts::find(&mut *tx, case.contract_id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    if old.status != ContractStatus::Active.to_string() {
        return Err(ServiceError::Conflict(format!(
            "the current contract is {}, not Active",
            old.status
        )));
    }
    if input.start_date <= old.end_date && input.start_date < old.start_date {
        return Err(ServiceError::validation(
            "the new contract cannot start before the current one",
        ));
    }
    let sequence = old.renewal_sequence + 1;
    let root = old.root_contract_id.unwrap_or(old.id);
    let number = match input
        .contract_number
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(n) => n.to_owned(),
        None => {
            let base = old
                .contract_number
                .split("-R")
                .next()
                .unwrap_or(&old.contract_number);
            format!("{base}-R{sequence}")
        }
    };
    if contracts::number_exists(&mut *tx, &number, None).await? {
        return Err(ServiceError::Conflict(format!(
            "contract number {number} is already in use"
        )));
    }
    let new_input = ContractInput {
        contract_number: number,
        tenant_id: old.tenant_id,
        building_id: old.building_id,
        unit_ids: old.unit_ids.clone(),
        // The renewal keeps the same tenants per unit unless the leasing team changes it later.
        unit_tenants: old
            .unit_ids
            .iter()
            .copied()
            .zip(old.unit_occupant_counts.iter().copied())
            .collect(),
        start_date: input.start_date,
        end_date: input.end_date,
        rent_terms: input.rent_terms.or(old.rent_terms.clone()),
        rent_amount_minor: input.rent_amount_minor.or(old.rent_amount_minor),
        assigned_employee_id: case.assigned_employee_id.or(old.assigned_employee_id),
        notes: input.notes,
    };
    // 1. Close the old contract first so the unit-availability check sees it as released.
    contracts::set_status(&mut *tx, old.id, &ContractStatus::Renewed.to_string()).await?;
    // 2. Create the linked new contract, Active from day one.
    let new_id = contracts::insert(
        &mut tx,
        &InsertContract {
            input: &new_input,
            status: &ContractStatus::Active.to_string(),
            previous_contract_id: Some(old.id),
            root_contract_id: Some(root),
            renewal_sequence: sequence,
            created_by: caller.user_id,
        },
    )
    .await?;
    // 3. Units stay Occupied, now pointing at the new contract.
    units::set_status_many(
        &mut *tx,
        &old.unit_ids,
        &UnitStatus::Occupied.to_string(),
        None,
    )
    .await?;
    // 4. Case → Renewal Completed with checklist ticks.
    renewals::set_outcome(&mut *tx, id, new_id).await?;
    renewals::tick_by_key(&mut *tx, id, "NEW_CONTRACT", caller.user_id).await?;
    renewals::tick_by_key(&mut *tx, id, "COMPLETED", caller.user_id).await?;
    renewals::set_status(&mut *tx, id, &RenewalStatus::RenewalCompleted.to_string()).await?;
    // 5. Audit everything.
    let new_row = contracts::find(&mut *tx, new_id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    let after = renewals::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("renewal case"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "contract",
        old.id,
        "RENEWED",
        Some(&old.status),
        Some(&serde_json::json!({ "status": "RENEWED", "renewedBy": new_id })),
    )
    .await?;
    audit_log::log(
        &mut *tx,
        caller,
        "contract",
        new_id,
        "CREATED",
        NONE,
        Some(&new_row),
    )
    .await?;
    audit_log::log(
        &mut *tx,
        caller,
        "renewal_case",
        id,
        "RENEWAL_COMPLETED",
        Some(&case.status),
        Some(&after.status),
    )
    .await?;
    tx.commit().await?;
    Ok((after, new_row))
}

// ---------------------------------------------------------------- checklist template (Admin)

pub async fn template(pool: &PgPool, caller: &Session) -> ServiceResult<Vec<TemplateItemRow>> {
    caller.require(Capability::ViewRenewals)?;
    Ok(renewals::template(pool).await?)
}

pub struct TemplateInput {
    pub id: Option<Uuid>,
    pub label: String,
    pub required: bool,
    pub active: bool,
}

pub async fn save_template(
    pool: &PgPool,
    caller: &Session,
    items: Vec<TemplateInput>,
) -> ServiceResult<Vec<TemplateItemRow>> {
    caller.require(Capability::ManageSettings)?;
    let cleaned: Vec<(Option<Uuid>, String, bool, bool)> = items
        .into_iter()
        .map(|i| (i.id, i.label.trim().to_owned(), i.required, i.active))
        .filter(|(_, label, _, _)| !label.is_empty())
        .collect();
    if cleaned.is_empty() {
        return Err(ServiceError::validation(
            "the checklist needs at least one item",
        ));
    }
    let mut tx = pool.begin().await?;
    let before = renewals::template(&mut *tx).await?;
    let inputs: Vec<TemplateItemInput<'_>> = cleaned
        .iter()
        .enumerate()
        .map(|(i, (id, label, required, active))| TemplateItemInput {
            id: *id,
            label,
            sort_order: (i as i32 + 1) * 10,
            required: *required,
            active: *active,
        })
        .collect();
    renewals::save_template(&mut tx, &inputs).await?;
    let after = renewals::template(&mut *tx).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "settings",
        Uuid::nil(),
        "CHECKLIST_TEMPLATE_UPDATED",
        Some(&before),
        Some(&after),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}
