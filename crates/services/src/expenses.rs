//! Expenses per unit and their split between occupants; aggregates for the dashboard.

use chrono::{Datelike, NaiveDate};
use renewal_core::{equal_split, shares_cover, Capability, ExpenseCategory, SplitMethod};
use renewal_db::expenses::{
    self, CategoryTotal, ExpenseFilter, ExpenseInput, ExpenseRow, GroupTotal, MonthTotal, ShareRow,
    SummaryScope,
};
use renewal_db::occupants::{self, OccupantRow};
use renewal_db::paging::{ListQuery, PageResult};
use renewal_db::{units, PgPool};
use uuid::Uuid;

use crate::audit_log::{self, NONE};
use crate::buildings::trim_opt;
use crate::error::{ServiceError, ServiceResult};
use crate::session::Session;

pub struct ExpenseDetail {
    pub expense: ExpenseRow,
    pub shares: Vec<ShareRow>,
    pub occupants: Vec<OccupantRow>,
}

pub struct Summary {
    pub scope: SummaryScope,
    pub total: i64,
    pub expense_count: i64,
    pub this_month: i64,
    pub last_month: i64,
    pub outstanding: i64,
    pub outstanding_shares: i64,
    pub monthly: Vec<MonthTotal>,
    pub by_building: Vec<GroupTotal>,
    pub by_unit: Vec<GroupTotal>,
    pub by_category: Vec<CategoryTotal>,
}

pub async fn list(
    pool: &PgPool,
    caller: &Session,
    f: &ExpenseFilter,
    q: &ListQuery,
) -> ServiceResult<PageResult<ExpenseRow>> {
    caller.require(Capability::ViewExpenses)?;
    Ok(expenses::list(pool, f, q).await?)
}

pub async fn get(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<ExpenseDetail> {
    caller.require(Capability::ViewExpenses)?;
    let expense = expenses::find(pool, id)
        .await?
        .ok_or(ServiceError::NotFound("expense"))?;
    let shares = expenses::shares(pool, id).await?;
    let occupants = occupants::present_on(pool, expense.unit_id, expense.expense_date).await?;
    Ok(ExpenseDetail {
        expense,
        shares,
        occupants,
    })
}

fn validate(input: &mut ExpenseInput) -> ServiceResult<()> {
    input.description = input.description.trim().to_owned();
    trim_opt(&mut input.vendor);
    trim_opt(&mut input.reference);
    trim_opt(&mut input.notes);
    if input.description.is_empty() {
        return Err(ServiceError::validation("a description is required"));
    }
    if input.amount_minor <= 0 {
        return Err(ServiceError::validation(
            "the amount must be greater than zero",
        ));
    }
    input
        .category
        .parse::<ExpenseCategory>()
        .map_err(|_| ServiceError::validation("invalid expense category"))?;
    input
        .split_method
        .parse::<SplitMethod>()
        .map_err(|_| ServiceError::validation("invalid split method"))?;
    if let (Some(a), Some(b)) = (input.period_start, input.period_end) {
        if b < a {
            return Err(ServiceError::validation(
                "the billing period end cannot be before its start",
            ));
        }
    }
    Ok(())
}

/// Equal shares between the occupants present on the expense date; error when there is nobody.
async fn equal_shares(
    tx: &mut renewal_db::PgConnection,
    unit_id: Uuid,
    on: NaiveDate,
    amount_minor: i64,
) -> ServiceResult<Vec<(Uuid, i64)>> {
    let people = occupants::present_on(&mut *tx, unit_id, on).await?;
    if people.is_empty() {
        return Err(ServiceError::validation(
            "no occupants are recorded for this unit on the expense date — add them first, or choose another split",
        ));
    }
    let amounts = equal_split(amount_minor, people.len());
    Ok(people.iter().zip(amounts).map(|(o, a)| (o.id, a)).collect())
}

pub async fn create(
    pool: &PgPool,
    caller: &Session,
    mut input: ExpenseInput,
) -> ServiceResult<ExpenseRow> {
    caller.require(Capability::ManageExpenses)?;
    validate(&mut input)?;
    units::find(pool, input.unit_id)
        .await?
        .ok_or(ServiceError::NotFound("unit"))?;
    let mut tx = pool.begin().await?;
    let id = expenses::insert(&mut *tx, &input, caller.user_id).await?;
    if input.split_method == SplitMethod::Equal.to_string() {
        let shares = equal_shares(
            &mut tx,
            input.unit_id,
            input.expense_date,
            input.amount_minor,
        )
        .await?;
        expenses::replace_shares(&mut tx, id, &shares).await?;
    }
    let row = expenses::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("expense"))?;
    audit_log::log(&mut *tx, caller, "expense", id, "CREATED", NONE, Some(&row)).await?;
    tx.commit().await?;
    Ok(row)
}

pub async fn update(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    mut input: ExpenseInput,
) -> ServiceResult<ExpenseRow> {
    caller.require(Capability::ManageExpenses)?;
    validate(&mut input)?;
    let mut tx = pool.begin().await?;
    let before = expenses::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("expense"))?;
    units::find(&mut *tx, input.unit_id)
        .await?
        .ok_or(ServiceError::NotFound("unit"))?;
    expenses::update(&mut *tx, id, &input).await?;
    // Keep the shares consistent with the new amount / method.
    match input
        .split_method
        .parse::<SplitMethod>()
        .unwrap_or(SplitMethod::None)
    {
        SplitMethod::None => expenses::replace_shares(&mut tx, id, &[]).await?,
        SplitMethod::Equal => {
            let shares = equal_shares(
                &mut tx,
                input.unit_id,
                input.expense_date,
                input.amount_minor,
            )
            .await?;
            expenses::replace_shares(&mut tx, id, &shares).await?;
        }
        SplitMethod::Custom => {
            // Existing custom shares stay unless the total changed; then they must be re-entered.
            let existing = expenses::shares(&mut *tx, id).await?;
            let sum: i64 = existing.iter().map(|s| s.amount_minor).sum();
            if before.amount_minor != input.amount_minor
                || before.unit_id != input.unit_id
                || sum != input.amount_minor
            {
                expenses::replace_shares(&mut tx, id, &[]).await?;
            }
        }
    }
    let after = expenses::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("expense"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "expense",
        id,
        "UPDATED",
        Some(&before),
        Some(&after),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

pub async fn delete(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<()> {
    caller.require(Capability::ManageExpenses)?;
    let mut tx = pool.begin().await?;
    let before = expenses::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("expense"))?;
    expenses::delete(&mut *tx, id).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "expense",
        id,
        "DELETED",
        Some(&before),
        NONE,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Splits the bill evenly between the occupants present on the expense date.
pub async fn split_equal(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
) -> ServiceResult<ExpenseDetail> {
    caller.require(Capability::ManageExpenses)?;
    let mut tx = pool.begin().await?;
    let mut e = expenses::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("expense"))?;
    let shares = equal_shares(&mut tx, e.unit_id, e.expense_date, e.amount_minor).await?;
    expenses::replace_shares(&mut tx, id, &shares).await?;
    if e.split_method != SplitMethod::Equal.to_string() {
        e.split_method = SplitMethod::Equal.to_string();
        let input = row_to_input(&e);
        expenses::update(&mut *tx, id, &input).await?;
    }
    audit_log::log(
        &mut *tx,
        caller,
        "expense",
        id,
        "SPLIT_EQUAL",
        NONE,
        Some(&shares.len()),
    )
    .await?;
    tx.commit().await?;
    get(pool, caller, id).await
}

/// Custom shares entered by hand; they must add up to the bill exactly.
pub async fn set_shares(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    shares: Vec<(Uuid, i64)>,
) -> ServiceResult<ExpenseDetail> {
    caller.require(Capability::ManageExpenses)?;
    let mut tx = pool.begin().await?;
    let mut e = expenses::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("expense"))?;
    let amounts: Vec<i64> = shares.iter().map(|(_, a)| *a).collect();
    if !shares_cover(e.amount_minor, &amounts) {
        return Err(ServiceError::validation(
            "the shares must be zero or more and add up to the full amount",
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for (occupant_id, _) in &shares {
        if !seen.insert(*occupant_id) {
            return Err(ServiceError::validation("an occupant is listed twice"));
        }
        let o = occupants::find(&mut *tx, *occupant_id)
            .await?
            .ok_or(ServiceError::NotFound("occupant"))?;
        if o.unit_id != e.unit_id {
            return Err(ServiceError::validation(
                "shares can only go to occupants of the same unit",
            ));
        }
    }
    expenses::replace_shares(&mut tx, id, &shares).await?;
    e.split_method = SplitMethod::Custom.to_string();
    expenses::update(&mut *tx, id, &row_to_input(&e)).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "expense",
        id,
        "SPLIT_CUSTOM",
        NONE,
        Some(&shares.len()),
    )
    .await?;
    tx.commit().await?;
    get(pool, caller, id).await
}

pub async fn settle(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    occupant_id: Uuid,
    settled: bool,
) -> ServiceResult<ExpenseDetail> {
    caller.require(Capability::ManageExpenses)?;
    let mut tx = pool.begin().await?;
    if !expenses::set_share_settled(&mut *tx, id, occupant_id, settled).await? {
        return Err(ServiceError::NotFound("expense share"));
    }
    let action = if settled {
        "SHARE_SETTLED"
    } else {
        "SHARE_UNSETTLED"
    };
    audit_log::log(
        &mut *tx,
        caller,
        "expense",
        id,
        action,
        NONE,
        Some(&occupant_id),
    )
    .await?;
    tx.commit().await?;
    get(pool, caller, id).await
}

fn row_to_input(e: &ExpenseRow) -> ExpenseInput {
    ExpenseInput {
        unit_id: e.unit_id,
        category: e.category.clone(),
        description: e.description.clone(),
        amount_minor: e.amount_minor,
        expense_date: e.expense_date,
        period_start: e.period_start,
        period_end: e.period_end,
        vendor: e.vendor.clone(),
        reference: e.reference.clone(),
        split_method: e.split_method.clone(),
        notes: e.notes.clone(),
    }
}

fn month_start(d: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap_or(d)
}

fn prev_month_start(d: NaiveDate) -> NaiveDate {
    let (y, m) = if d.month() == 1 {
        (d.year() - 1, 12)
    } else {
        (d.year(), d.month() - 1)
    };
    NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(d)
}

/// Dashboard numbers for a scope (everything / one building / one unit) and date range.
pub async fn summary(
    pool: &PgPool,
    caller: &Session,
    building_id: Option<Uuid>,
    unit_id: Option<Uuid>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> ServiceResult<Summary> {
    caller.require(Capability::ViewExpenses)?;
    let today = renewal_db::today(pool).await?;
    // Default window: the last 12 calendar months including this one.
    let default_from = {
        let mut d = month_start(today);
        for _ in 0..11 {
            d = prev_month_start(d);
        }
        d
    };
    let scope = SummaryScope {
        building_id,
        unit_id,
        from: from.unwrap_or(default_from),
        to: to.unwrap_or(today),
    };
    if scope.to < scope.from {
        return Err(ServiceError::validation("the date range is reversed"));
    }
    let monthly = expenses::monthly(pool, &scope).await?;
    let by_building = expenses::by_building(pool, &scope).await?;
    let by_unit = expenses::by_unit(pool, &scope, 12).await?;
    let by_category = expenses::by_category(pool, &scope).await?;
    let (outstanding, outstanding_shares) = expenses::outstanding(pool, &scope).await?;
    let total: i64 = monthly.iter().map(|m| m.amount_minor).sum();
    let expense_count: i64 = monthly.iter().map(|m| m.expense_count).sum();

    // This month / last month are always calendar months, regardless of the range.
    let this_start = month_start(today);
    let last_start = prev_month_start(this_start);
    let calendar = SummaryScope {
        building_id,
        unit_id,
        from: last_start,
        to: today,
    };
    let recent = expenses::monthly(pool, &calendar).await?;
    let pick = |d: NaiveDate| {
        recent
            .iter()
            .find(|m| m.month == d)
            .map(|m| m.amount_minor)
            .unwrap_or(0)
    };
    Ok(Summary {
        this_month: pick(this_start),
        last_month: pick(last_start),
        scope,
        total,
        expense_count,
        outstanding,
        outstanding_shares,
        monthly,
        by_building,
        by_unit,
        by_category,
    })
}
