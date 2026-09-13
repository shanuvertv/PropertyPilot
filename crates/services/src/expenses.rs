//! Expenses per unit, split equally between the people living there; aggregates for
//! the dashboard.

use chrono::{Datelike, NaiveDate};
use renewal_core::{equal_split, Capability, ExpenseCategory, SplitMethod};
use renewal_db::expenses::{
    self, CategoryTotal, ExpenseFilter, ExpenseInput, ExpenseRow, GroupTotal, MonthTotal,
    SummaryScope,
};
use renewal_db::paging::{ListQuery, PageResult};
use renewal_db::{units, PgPool};
use uuid::Uuid;

use crate::audit_log::{self, NONE};
use crate::buildings::trim_opt;
use crate::error::{ServiceError, ServiceResult};
use crate::session::Session;

/// Upper bound on how many people a bill can be split between (sanity, not policy).
pub const MAX_SPLIT: i32 = 500;

/// One person's equal share of a split bill.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Share {
    /// 1-based position; the first shares carry the rounding remainder.
    pub index: i32,
    pub amount_minor: i64,
    /// Paid: the first `settled_count` shares count as paid.
    pub settled: bool,
}

pub struct ExpenseDetail {
    pub expense: ExpenseRow,
    pub shares: Vec<Share>,
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

/// The equal shares of an expense, marked paid in order.
pub fn shares_of(e: &ExpenseRow) -> Vec<Share> {
    if e.split_method != SplitMethod::Equal.to_string() || e.split_count <= 0 {
        return Vec::new();
    }
    equal_split(e.amount_minor, e.split_count as usize)
        .into_iter()
        .enumerate()
        .map(|(i, amount_minor)| Share {
            index: i as i32 + 1,
            amount_minor,
            settled: (i as i32) < e.settled_count,
        })
        .collect()
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
    let shares = shares_of(&expense);
    Ok(ExpenseDetail { expense, shares })
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
    if !(0..=MAX_SPLIT).contains(&input.split_count) {
        return Err(ServiceError::validation(
            "the number of people to split between is out of range",
        ));
    }
    if let (Some(a), Some(b)) = (input.period_start, input.period_end) {
        if b < a {
            return Err(ServiceError::validation(
                "the billing period end cannot be before its start",
            ));
        }
    }
    Ok(())
}

/// Resolves the split: `EQUAL` needs a head count — the one given, else the unit's.
async fn resolve_split(
    ex: impl renewal_db::sqlx::PgExecutor<'_>,
    input: &mut ExpenseInput,
) -> ServiceResult<()> {
    let unit = units::find(ex, input.unit_id)
        .await?
        .ok_or(ServiceError::NotFound("unit"))?;
    if input.split_method == SplitMethod::Equal.to_string() {
        if input.split_count == 0 {
            input.split_count = unit.occupant_count;
        }
        if input.split_count == 0 {
            return Err(ServiceError::validation(
                "set the number of tenants living in this unit first, or record the expense without a split",
            ));
        }
    } else {
        input.split_count = 0;
    }
    Ok(())
}

pub async fn create(
    pool: &PgPool,
    caller: &Session,
    mut input: ExpenseInput,
) -> ServiceResult<ExpenseRow> {
    caller.require(Capability::ManageExpenses)?;
    validate(&mut input)?;
    let mut tx = pool.begin().await?;
    resolve_split(&mut *tx, &mut input).await?;
    let id = expenses::insert(&mut *tx, &input, caller.user_id).await?;
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
    resolve_split(&mut *tx, &mut input).await?;
    expenses::update(&mut *tx, id, &input).await?;
    // A different bill means the payments collected so far no longer apply.
    if before.amount_minor != input.amount_minor {
        expenses::set_settled_count(&mut *tx, id, 0).await?;
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

/// Splits the bill evenly between `count` people (default: the unit's head count).
/// Payments collected so far are reset — the shares changed.
pub async fn split_equal(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    count: Option<i32>,
) -> ServiceResult<ExpenseDetail> {
    caller.require(Capability::ManageExpenses)?;
    let mut tx = pool.begin().await?;
    let e = expenses::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("expense"))?;
    let mut input = row_to_input(&e);
    input.split_method = SplitMethod::Equal.to_string();
    input.split_count = count.unwrap_or(0);
    validate(&mut input)?;
    resolve_split(&mut *tx, &mut input).await?;
    expenses::update(&mut *tx, id, &input).await?;
    expenses::set_settled_count(&mut *tx, id, 0).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "expense",
        id,
        "SPLIT_EQUAL",
        Some(&e.split_count),
        Some(&input.split_count),
    )
    .await?;
    tx.commit().await?;
    get(pool, caller, id).await
}

/// Records how many of the people have paid their share (0 ..= split_count).
pub async fn settle(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    settled_count: i32,
) -> ServiceResult<ExpenseDetail> {
    caller.require(Capability::ManageExpenses)?;
    let mut tx = pool.begin().await?;
    let e = expenses::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("expense"))?;
    if e.split_method != SplitMethod::Equal.to_string() {
        return Err(ServiceError::validation(
            "this expense is not split, so there is nothing to collect",
        ));
    }
    if !(0..=e.split_count).contains(&settled_count) {
        return Err(ServiceError::validation(
            "the number paid must be between zero and the number of people",
        ));
    }
    expenses::set_settled_count(&mut *tx, id, settled_count).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "expense",
        id,
        "SETTLED_COUNT_CHANGED",
        Some(&e.settled_count),
        Some(&settled_count),
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
        split_count: e.split_count,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn row(method: &str, amount: i64, split: i32, settled: i32) -> ExpenseRow {
        ExpenseRow {
            id: Uuid::nil(),
            unit_id: Uuid::nil(),
            unit_number: "101".into(),
            building_id: Uuid::nil(),
            building_name: "B".into(),
            category: "WATER".into(),
            description: "bill".into(),
            amount_minor: amount,
            expense_date: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            period_start: None,
            period_end: None,
            vendor: None,
            reference: None,
            split_method: method.into(),
            notes: None,
            split_count: split,
            settled_count: settled,
            created_by_name: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn shares_follow_the_split_and_the_paid_count() {
        let s = shares_of(&row("EQUAL", 10_000, 3, 1));
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].amount_minor, 3_334);
        assert!(s[0].settled);
        assert!(!s[1].settled);
        assert_eq!(s.iter().map(|x| x.amount_minor).sum::<i64>(), 10_000);
        assert!(shares_of(&row("NONE", 10_000, 3, 1)).is_empty());
        assert!(shares_of(&row("EQUAL", 10_000, 0, 0)).is_empty());
    }
}
