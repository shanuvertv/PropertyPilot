//! Expenses per unit, their split between occupants, and the aggregates behind the
//! expenses dashboard. Amounts are minor units (fils) as `i64`.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgExecutor, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::paging::{ListQuery, PageResult};
use crate::DbResult;

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct ExpenseRow {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub unit_number: String,
    pub building_id: Uuid,
    pub building_name: String,
    pub category: String,
    pub description: String,
    pub amount_minor: i64,
    pub expense_date: NaiveDate,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub vendor: Option<String>,
    pub reference: Option<String>,
    pub split_method: String,
    pub notes: Option<String>,
    pub share_count: i64,
    pub settled_count: i64,
    pub created_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct ShareRow {
    pub expense_id: Uuid,
    pub occupant_id: Uuid,
    pub occupant_name: String,
    pub bed_label: Option<String>,
    pub amount_minor: i64,
    pub settled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct ExpenseInput {
    pub unit_id: Uuid,
    pub category: String,
    pub description: String,
    pub amount_minor: i64,
    pub expense_date: NaiveDate,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub vendor: Option<String>,
    pub reference: Option<String>,
    pub split_method: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ExpenseFilter {
    pub unit_id: Option<Uuid>,
    pub building_id: Option<Uuid>,
    pub category: Option<String>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// Only expenses with at least one unsettled share.
    pub outstanding: Option<bool>,
}

const SELECT: &str = "SELECT x.id, x.unit_id, u.unit_number, u.building_id, b.name AS building_name, x.category,
       x.description, x.amount_minor, x.expense_date, x.period_start, x.period_end, x.vendor, x.reference,
       x.split_method, x.notes,
       (SELECT count(*) FROM expense_shares s WHERE s.expense_id = x.id) AS share_count,
       (SELECT count(*) FROM expense_shares s WHERE s.expense_id = x.id AND s.settled_at IS NOT NULL) AS settled_count,
       cu.name AS created_by_name, x.created_at, x.updated_at
  FROM expenses x
  JOIN units u ON u.id = x.unit_id
  JOIN buildings b ON b.id = u.building_id
  LEFT JOIN users cu ON cu.id = x.created_by";

const SORTS: &[(&str, &str)] = &[
    ("expense_date", "x.expense_date, x.created_at"),
    ("amount", "x.amount_minor"),
    ("category", "x.category, x.expense_date"),
    ("unit", "b.name, u.unit_number, x.expense_date"),
    ("building", "b.name, u.unit_number, x.expense_date"),
    ("created_at", "x.created_at"),
];

fn push_filters(qb: &mut QueryBuilder<Postgres>, f: &ExpenseFilter, like: &Option<String>) {
    qb.push(" WHERE TRUE");
    if let Some(u) = f.unit_id {
        qb.push(" AND x.unit_id = ").push_bind(u);
    }
    if let Some(bid) = f.building_id {
        qb.push(" AND u.building_id = ").push_bind(bid);
    }
    if let Some(c) = &f.category {
        qb.push(" AND x.category = ").push_bind(c.clone());
    }
    if let Some(d) = f.from {
        qb.push(" AND x.expense_date >= ").push_bind(d);
    }
    if let Some(d) = f.to {
        qb.push(" AND x.expense_date <= ").push_bind(d);
    }
    if f.outstanding == Some(true) {
        qb.push(" AND EXISTS (SELECT 1 FROM expense_shares s WHERE s.expense_id = x.id AND s.settled_at IS NULL)");
    }
    if let Some(p) = like {
        qb.push(" AND (x.description ILIKE ")
            .push_bind(p.clone())
            .push(" OR x.vendor ILIKE ")
            .push_bind(p.clone())
            .push(" OR x.reference ILIKE ")
            .push_bind(p.clone())
            .push(" OR u.unit_number ILIKE ")
            .push_bind(p.clone())
            .push(" OR b.name ILIKE ")
            .push_bind(p.clone())
            .push(")");
    }
}

pub async fn list(
    pool: &PgPool,
    f: &ExpenseFilter,
    q: &ListQuery,
) -> DbResult<PageResult<ExpenseRow>> {
    let like = q.like();
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(SELECT.replacen(
        "SELECT ",
        "SELECT count(*) OVER () AS total_count, ",
        1,
    ));
    push_filters(&mut qb, f, &like);
    qb.push(q.order_by(SORTS))
        .push(" LIMIT ")
        .push_bind(q.page_size)
        .push(" OFFSET ")
        .push_bind(q.offset());
    let rows = qb.build().fetch_all(pool).await?;
    let total: i64 = rows.first().map(|r| r.get("total_count")).unwrap_or(0);
    let items = rows
        .iter()
        .map(ExpenseRow::from_row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PageResult { items, total })
}

pub async fn find<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<Option<ExpenseRow>> {
    sqlx::query_as(&format!("{SELECT} WHERE x.id = $1"))
        .bind(id)
        .fetch_optional(ex)
        .await
}

pub async fn insert<'e>(
    ex: impl PgExecutor<'e>,
    i: &ExpenseInput,
    created_by: Uuid,
) -> DbResult<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO expenses (unit_id, category, description, amount_minor, expense_date, period_start, period_end,
                               vendor, reference, split_method, notes, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING id",
    )
    .bind(i.unit_id)
    .bind(&i.category)
    .bind(&i.description)
    .bind(i.amount_minor)
    .bind(i.expense_date)
    .bind(i.period_start)
    .bind(i.period_end)
    .bind(&i.vendor)
    .bind(&i.reference)
    .bind(&i.split_method)
    .bind(&i.notes)
    .bind(created_by)
    .fetch_one(ex)
    .await
}

pub async fn update<'e>(ex: impl PgExecutor<'e>, id: Uuid, i: &ExpenseInput) -> DbResult<()> {
    sqlx::query(
        "UPDATE expenses SET unit_id = $2, category = $3, description = $4, amount_minor = $5, expense_date = $6,
                period_start = $7, period_end = $8, vendor = $9, reference = $10, split_method = $11, notes = $12,
                updated_at = now()
          WHERE id = $1",
    )
    .bind(id)
    .bind(i.unit_id)
    .bind(&i.category)
    .bind(&i.description)
    .bind(i.amount_minor)
    .bind(i.expense_date)
    .bind(i.period_start)
    .bind(i.period_end)
    .bind(&i.vendor)
    .bind(&i.reference)
    .bind(&i.split_method)
    .bind(&i.notes)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn delete<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<()> {
    sqlx::query("DELETE FROM expenses WHERE id = $1")
        .bind(id)
        .execute(ex)
        .await
        .map(|_| ())
}

// ---------------------------------------------------------------- shares

pub async fn shares<'e>(ex: impl PgExecutor<'e>, expense_id: Uuid) -> DbResult<Vec<ShareRow>> {
    sqlx::query_as(
        "SELECT s.expense_id, s.occupant_id, o.full_name AS occupant_name, o.bed_label, s.amount_minor, s.settled_at
           FROM expense_shares s JOIN occupants o ON o.id = s.occupant_id
          WHERE s.expense_id = $1
          ORDER BY o.bed_label NULLS LAST, o.full_name",
    )
    .bind(expense_id)
    .fetch_all(ex)
    .await
}

/// Replaces all shares of an expense (settled flags are reset — the amounts changed).
pub async fn replace_shares(
    tx: &mut sqlx::PgConnection,
    expense_id: Uuid,
    shares: &[(Uuid, i64)],
) -> DbResult<()> {
    sqlx::query("DELETE FROM expense_shares WHERE expense_id = $1")
        .bind(expense_id)
        .execute(&mut *tx)
        .await?;
    for (occupant_id, amount) in shares {
        sqlx::query(
            "INSERT INTO expense_shares (expense_id, occupant_id, amount_minor) VALUES ($1, $2, $3)",
        )
        .bind(expense_id)
        .bind(occupant_id)
        .bind(amount)
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

pub async fn set_share_settled<'e>(
    ex: impl PgExecutor<'e>,
    expense_id: Uuid,
    occupant_id: Uuid,
    settled: bool,
) -> DbResult<bool> {
    let res = sqlx::query(
        "UPDATE expense_shares SET settled_at = CASE WHEN $3 THEN now() ELSE NULL END
          WHERE expense_id = $1 AND occupant_id = $2",
    )
    .bind(expense_id)
    .bind(occupant_id)
    .bind(settled)
    .execute(ex)
    .await?;
    Ok(res.rows_affected() > 0)
}

// ---------------------------------------------------------------- aggregates

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct MonthTotal {
    pub month: NaiveDate,
    pub amount_minor: i64,
    pub expense_count: i64,
}

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct GroupTotal {
    pub id: Uuid,
    pub label: String,
    pub sublabel: Option<String>,
    pub amount_minor: i64,
    pub expense_count: i64,
}

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct CategoryTotal {
    pub category: String,
    pub amount_minor: i64,
    pub expense_count: i64,
}

#[derive(Debug, Clone, Default)]
pub struct SummaryScope {
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub from: NaiveDate,
    pub to: NaiveDate,
}

fn scope_sql(prefix: &str) -> String {
    format!(
        "{prefix} x.expense_date >= $1 AND x.expense_date <= $2
         AND ($3::uuid IS NULL OR u.building_id = $3) AND ($4::uuid IS NULL OR x.unit_id = $4)"
    )
}

/// Totals per calendar month in the range (months without expenses are absent).
pub async fn monthly<'e>(ex: impl PgExecutor<'e>, s: &SummaryScope) -> DbResult<Vec<MonthTotal>> {
    sqlx::query_as(&format!(
        "SELECT date_trunc('month', x.expense_date)::date AS month, sum(x.amount_minor)::bigint AS amount_minor,
                count(*) AS expense_count
           FROM expenses x JOIN units u ON u.id = x.unit_id
          {} GROUP BY 1 ORDER BY 1",
        scope_sql("WHERE")
    ))
    .bind(s.from)
    .bind(s.to)
    .bind(s.building_id)
    .bind(s.unit_id)
    .fetch_all(ex)
    .await
}

pub async fn by_building<'e>(
    ex: impl PgExecutor<'e>,
    s: &SummaryScope,
) -> DbResult<Vec<GroupTotal>> {
    sqlx::query_as(&format!(
        "SELECT b.id, b.name AS label, b.code AS sublabel, sum(x.amount_minor)::bigint AS amount_minor,
                count(*) AS expense_count
           FROM expenses x JOIN units u ON u.id = x.unit_id JOIN buildings b ON b.id = u.building_id
          {} GROUP BY b.id, b.name, b.code ORDER BY amount_minor DESC",
        scope_sql("WHERE")
    ))
    .bind(s.from)
    .bind(s.to)
    .bind(s.building_id)
    .bind(s.unit_id)
    .fetch_all(ex)
    .await
}

pub async fn by_unit<'e>(
    ex: impl PgExecutor<'e>,
    s: &SummaryScope,
    limit: i64,
) -> DbResult<Vec<GroupTotal>> {
    sqlx::query_as(&format!(
        "SELECT u.id, u.unit_number AS label, b.name AS sublabel, sum(x.amount_minor)::bigint AS amount_minor,
                count(*) AS expense_count
           FROM expenses x JOIN units u ON u.id = x.unit_id JOIN buildings b ON b.id = u.building_id
          {} GROUP BY u.id, u.unit_number, b.name ORDER BY amount_minor DESC LIMIT $5",
        scope_sql("WHERE")
    ))
    .bind(s.from)
    .bind(s.to)
    .bind(s.building_id)
    .bind(s.unit_id)
    .bind(limit)
    .fetch_all(ex)
    .await
}

pub async fn by_category<'e>(
    ex: impl PgExecutor<'e>,
    s: &SummaryScope,
) -> DbResult<Vec<CategoryTotal>> {
    sqlx::query_as(&format!(
        "SELECT x.category, sum(x.amount_minor)::bigint AS amount_minor, count(*) AS expense_count
           FROM expenses x JOIN units u ON u.id = x.unit_id
          {} GROUP BY x.category ORDER BY amount_minor DESC",
        scope_sql("WHERE")
    ))
    .bind(s.from)
    .bind(s.to)
    .bind(s.building_id)
    .bind(s.unit_id)
    .fetch_all(ex)
    .await
}

/// Sum of shares not yet settled within the scope.
pub async fn outstanding<'e>(ex: impl PgExecutor<'e>, s: &SummaryScope) -> DbResult<(i64, i64)> {
    let row: (i64, i64) = sqlx::query_as(&format!(
        "SELECT coalesce(sum(sh.amount_minor), 0)::bigint, count(*)
           FROM expense_shares sh JOIN expenses x ON x.id = sh.expense_id JOIN units u ON u.id = x.unit_id
          {} AND sh.settled_at IS NULL",
        scope_sql("WHERE")
    ))
    .bind(s.from)
    .bind(s.to)
    .bind(s.building_id)
    .bind(s.unit_id)
    .fetch_one(ex)
    .await?;
    Ok(row)
}
