//! Post-dated rent cheques on a contract, and the deposit reminders the daily sweep sends.

use chrono::NaiveDate;
use renewal_core::{plan_cheques, spacing_months, Capability, ChequeStatus};
use renewal_db::cheques::{self, ChequeFilter, ChequeInput, ChequeRow, ChequeSummaryRow};
use renewal_db::paging::{ListQuery, PageResult};
use renewal_db::{contracts, settings, PgPool};
use uuid::Uuid;

use crate::audit_log::{self, NONE};
use crate::buildings::trim_opt;
use crate::error::{ServiceError, ServiceResult};
use crate::notifications::{audience, notify_many};
use crate::session::Session;

/// Settings key: how many days before the cheque date the reminder goes out.
pub const REMINDER_DAYS_KEY: &str = "cheques.reminderDaysBefore";
pub const MAX_CHEQUES: usize = 60;

pub struct GenerateInput {
    pub count: usize,
    /// Date of the first cheque; defaults to the contract start.
    pub first_date: Option<NaiveDate>,
    /// Months between cheques; defaults to spreading them over the contract period.
    pub every_months: Option<u32>,
    /// Total to split; defaults to the contract's rent.
    pub total_minor: Option<i64>,
    pub bank_name: Option<String>,
    /// Remove the contract's PENDING cheques first.
    pub replace_pending: bool,
}

pub async fn list(
    pool: &PgPool,
    caller: &Session,
    f: &ChequeFilter,
    q: &ListQuery,
) -> ServiceResult<PageResult<ChequeRow>> {
    caller.require(Capability::ViewContracts)?;
    Ok(cheques::list(pool, f, q).await?)
}

pub async fn for_contract(
    pool: &PgPool,
    caller: &Session,
    contract_id: Uuid,
) -> ServiceResult<Vec<ChequeRow>> {
    caller.require(Capability::ViewContracts)?;
    Ok(cheques::for_contract(pool, contract_id).await?)
}

pub async fn get(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<ChequeRow> {
    caller.require(Capability::ViewContracts)?;
    cheques::find(pool, id)
        .await?
        .ok_or(ServiceError::NotFound("cheque"))
}

pub async fn summary(pool: &PgPool, caller: &Session) -> ServiceResult<ChequeSummaryRow> {
    caller.require(Capability::ViewContracts)?;
    Ok(cheques::summary(pool).await?)
}

fn validate(input: &mut ChequeInput) -> ServiceResult<()> {
    trim_opt(&mut input.cheque_number);
    trim_opt(&mut input.bank_name);
    trim_opt(&mut input.notes);
    if input.amount_minor <= 0 {
        return Err(ServiceError::validation(
            "the cheque amount must be greater than zero",
        ));
    }
    Ok(())
}

pub async fn create(
    pool: &PgPool,
    caller: &Session,
    contract_id: Uuid,
    mut input: ChequeInput,
) -> ServiceResult<ChequeRow> {
    caller.require(Capability::ManageContracts)?;
    validate(&mut input)?;
    contracts::find(pool, contract_id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    let mut tx = pool.begin().await?;
    let id = cheques::insert(&mut *tx, contract_id, &input, caller.user_id).await?;
    let row = cheques::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("cheque"))?;
    audit_log::log(&mut *tx, caller, "cheque", id, "CREATED", NONE, Some(&row)).await?;
    tx.commit().await?;
    Ok(row)
}

pub async fn update(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    mut input: ChequeInput,
) -> ServiceResult<ChequeRow> {
    caller.require(Capability::ManageContracts)?;
    validate(&mut input)?;
    let mut tx = pool.begin().await?;
    let before = cheques::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("cheque"))?;
    cheques::update(&mut *tx, id, &input).await?;
    cheques::renumber(&mut *tx, before.contract_id).await?;
    let after = cheques::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("cheque"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "cheque",
        id,
        "UPDATED",
        Some(&before),
        Some(&after),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

/// Deposited / cleared / bounced / cancelled, or back to pending.
pub async fn set_status(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    status: ChequeStatus,
) -> ServiceResult<ChequeRow> {
    caller.require(Capability::ManageContracts)?;
    let mut tx = pool.begin().await?;
    let before = cheques::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("cheque"))?;
    cheques::set_status(&mut *tx, id, &status.to_string()).await?;
    let after = cheques::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("cheque"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "cheque",
        id,
        "STATUS_CHANGED",
        Some(&before.status),
        Some(&after.status),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

pub async fn delete(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<()> {
    caller.require(Capability::ManageContracts)?;
    let mut tx = pool.begin().await?;
    let before = cheques::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("cheque"))?;
    cheques::delete(&mut *tx, id).await?;
    cheques::renumber(&mut *tx, before.contract_id).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "cheque",
        id,
        "DELETED",
        Some(&before),
        NONE,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Splits the rent into N cheques spread over the contract period (equal amounts, the
/// rounding remainder on the first ones); each can be edited afterwards.
pub async fn generate(
    pool: &PgPool,
    caller: &Session,
    contract_id: Uuid,
    input: GenerateInput,
) -> ServiceResult<Vec<ChequeRow>> {
    caller.require(Capability::ManageContracts)?;
    let contract = contracts::find(pool, contract_id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    if !(1..=MAX_CHEQUES).contains(&input.count) {
        return Err(ServiceError::validation(format!(
            "the number of cheques must be between 1 and {MAX_CHEQUES}"
        )));
    }
    let total = match input.total_minor.or(contract.rent_amount_minor) {
        Some(t) if t > 0 => t,
        _ => {
            return Err(ServiceError::validation(
                "enter the total to split, or set the rent on the contract first",
            ))
        }
    };
    let first = input.first_date.unwrap_or(contract.start_date);
    let every = input
        .every_months
        .unwrap_or_else(|| spacing_months(contract.start_date, contract.end_date, input.count));
    let mut bank = input.bank_name;
    trim_opt(&mut bank);
    let plan = plan_cheques(total, input.count, first, every);

    let mut tx = pool.begin().await?;
    if input.replace_pending {
        cheques::delete_pending(&mut *tx, contract_id).await?;
    }
    for c in &plan {
        let row = ChequeInput {
            cheque_number: None,
            bank_name: bank.clone(),
            amount_minor: c.amount_minor,
            due_date: c.due_date,
            notes: None,
        };
        cheques::insert(&mut *tx, contract_id, &row, caller.user_id).await?;
    }
    cheques::renumber(&mut *tx, contract_id).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "contract",
        contract_id,
        "CHEQUES_GENERATED",
        NONE,
        Some(&serde_json::json!({ "count": plan.len(), "totalMinor": total, "first": first, "everyMonths": every })),
    )
    .await?;
    tx.commit().await?;
    Ok(cheques::for_contract(pool, contract_id).await?)
}

// ---------------------------------------------------------------- reminders (daily sweep)

pub async fn reminder_days(pool: &PgPool) -> ServiceResult<i64> {
    Ok(settings::get::<i64>(pool, REMINDER_DAYS_KEY)
        .await?
        .unwrap_or(3)
        .clamp(0, 60))
}

fn fmt_date(d: NaiveDate) -> String {
    d.format("%d %b %Y").to_string()
}

fn describe(c: &ChequeRow) -> String {
    let number = c
        .cheque_number
        .as_deref()
        .map(|n| format!("cheque {n}"))
        .unwrap_or_else(|| format!("cheque {}", c.seq));
    format!(
        "{} · {} {} — {} of AED {} dated {}",
        c.tenant_name,
        c.building_name,
        c.unit_numbers,
        number,
        renewal_core::format_minor(c.amount_minor),
        fmt_date(c.due_date)
    )
}

/// Notifies the contract's assigned employee (or the Admins) N days before a cheque's
/// date, again on the day, and once when it has gone overdue without being deposited.
/// Returns how many notifications were created; safe to run several times a day.
pub async fn send_reminders(
    pool: &PgPool,
    today: NaiveDate,
    admins: &[Uuid],
) -> ServiceResult<usize> {
    let days = reminder_days(pool).await?;
    let mut created = 0;
    let mut batches: Vec<(Vec<ChequeRow>, &str, String)> = Vec::new();
    if days > 0 {
        let ahead = today + chrono::Duration::days(days);
        batches.push((
            cheques::pending_due_on(pool, ahead).await?,
            "CHEQUE_DUE",
            format!("Cheque to deposit in {days} days"),
        ));
    }
    batches.push((
        cheques::pending_due_on(pool, today).await?,
        "CHEQUE_DUE",
        "Cheque to deposit today".to_owned(),
    ));
    batches.push((
        cheques::overdue(pool).await?,
        "CHEQUE_OVERDUE",
        "Cheque not deposited".to_owned(),
    ));
    for (rows, kind, headline) in batches {
        for c in rows {
            let recipients = audience(c.assigned_employee_id, admins);
            let title = format!("{headline}: {}", c.tenant_name);
            let body = describe(&c);
            let dedupe = if kind == "CHEQUE_OVERDUE" {
                format!("cheque-overdue:{}", c.id)
            } else {
                format!("cheque-due:{}:{}", c.id, today)
            };
            created += notify_many(
                pool,
                &recipients,
                kind,
                &title,
                Some(&body),
                "contract",
                c.contract_id,
                Some(&dedupe),
            )
            .await?;
        }
    }
    Ok(created)
}
