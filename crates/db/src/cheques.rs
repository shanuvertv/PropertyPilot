//! Post-dated rent cheques per contract and the queries behind the deposit reminders.
//! Amounts are minor units (fils) as `i64`.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgExecutor, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::paging::{ListQuery, PageResult};
use crate::DbResult;

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct ChequeRow {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub contract_number: String,
    pub contract_status: String,
    pub tenant_id: Uuid,
    pub tenant_name: String,
    pub building_id: Uuid,
    pub building_name: String,
    pub unit_numbers: String,
    pub assigned_employee_id: Option<Uuid>,
    pub seq: i32,
    pub cheque_number: Option<String>,
    pub bank_name: Option<String>,
    pub amount_minor: i64,
    pub due_date: NaiveDate,
    pub status: String,
    pub status_changed_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    /// Negative once the cheque date has passed.
    pub days_until_due: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct ChequeInput {
    pub cheque_number: Option<String>,
    pub bank_name: Option<String>,
    pub amount_minor: i64,
    pub due_date: NaiveDate,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ChequeFilter {
    pub contract_id: Option<Uuid>,
    pub building_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub status: Option<Vec<String>>,
    pub due_from: Option<NaiveDate>,
    pub due_to: Option<NaiveDate>,
    /// Only PENDING cheques whose date has passed.
    pub overdue: Option<bool>,
}

const SELECT: &str = "SELECT q.id, q.contract_id, c.contract_number, c.status AS contract_status, c.tenant_id, t.name AS tenant_name,
       c.building_id, b.name AS building_name,
       (SELECT COALESCE(string_agg(u.unit_number, ', ' ORDER BY u.unit_number), '')
          FROM contract_units cu JOIN units u ON u.id = cu.unit_id WHERE cu.contract_id = c.id) AS unit_numbers,
       c.assigned_employee_id,
       q.seq, q.cheque_number, q.bank_name, q.amount_minor, q.due_date, q.status, q.status_changed_at, q.notes,
       (q.due_date - CURRENT_DATE)::int AS days_until_due, q.created_at, q.updated_at
  FROM cheques q
  JOIN contracts c ON c.id = q.contract_id
  JOIN tenants t ON t.id = c.tenant_id
  JOIN buildings b ON b.id = c.building_id";

const SORTS: &[(&str, &str)] = &[
    ("due_date", "q.due_date, q.seq"),
    ("amount", "q.amount_minor"),
    ("tenant_name", "t.name, q.due_date"),
    ("contract_number", "c.contract_number, q.seq"),
    ("status", "q.status, q.due_date"),
];

fn push_filters(qb: &mut QueryBuilder<Postgres>, f: &ChequeFilter, like: &Option<String>) {
    qb.push(" WHERE TRUE");
    if let Some(c) = f.contract_id {
        qb.push(" AND q.contract_id = ").push_bind(c);
    }
    if let Some(b) = f.building_id {
        qb.push(" AND c.building_id = ").push_bind(b);
    }
    if let Some(t) = f.tenant_id {
        qb.push(" AND c.tenant_id = ").push_bind(t);
    }
    if let Some(s) = &f.status {
        qb.push(" AND q.status = ANY(")
            .push_bind(s.clone())
            .push(")");
    }
    if let Some(d) = f.due_from {
        qb.push(" AND q.due_date >= ").push_bind(d);
    }
    if let Some(d) = f.due_to {
        qb.push(" AND q.due_date <= ").push_bind(d);
    }
    if f.overdue == Some(true) {
        qb.push(" AND q.status = 'PENDING' AND q.due_date < CURRENT_DATE");
    }
    if let Some(p) = like {
        qb.push(" AND (q.cheque_number ILIKE ")
            .push_bind(p.clone())
            .push(" OR q.bank_name ILIKE ")
            .push_bind(p.clone())
            .push(" OR t.name ILIKE ")
            .push_bind(p.clone())
            .push(" OR c.contract_number ILIKE ")
            .push_bind(p.clone())
            .push(" OR b.name ILIKE ")
            .push_bind(p.clone())
            .push(")");
    }
}

pub async fn list(
    pool: &PgPool,
    f: &ChequeFilter,
    q: &ListQuery,
) -> DbResult<PageResult<ChequeRow>> {
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
        .map(ChequeRow::from_row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PageResult { items, total })
}

pub async fn for_contract<'e>(
    ex: impl PgExecutor<'e>,
    contract_id: Uuid,
) -> DbResult<Vec<ChequeRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE q.contract_id = $1 ORDER BY q.seq, q.due_date"
    ))
    .bind(contract_id)
    .fetch_all(ex)
    .await
}

pub async fn find<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<Option<ChequeRow>> {
    sqlx::query_as(&format!("{SELECT} WHERE q.id = $1"))
        .bind(id)
        .fetch_optional(ex)
        .await
}

/// Appends a cheque as the contract's next in sequence.
pub async fn insert<'e>(
    ex: impl PgExecutor<'e>,
    contract_id: Uuid,
    i: &ChequeInput,
    created_by: Uuid,
) -> DbResult<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO cheques (contract_id, seq, cheque_number, bank_name, amount_minor, due_date, notes, created_by)
         VALUES ($1, (SELECT COALESCE(max(seq), 0) + 1 FROM cheques WHERE contract_id = $1), $2, $3, $4, $5, $6, $7)
         RETURNING id",
    )
    .bind(contract_id)
    .bind(&i.cheque_number)
    .bind(&i.bank_name)
    .bind(i.amount_minor)
    .bind(i.due_date)
    .bind(&i.notes)
    .bind(created_by)
    .fetch_one(ex)
    .await
}

pub async fn update<'e>(ex: impl PgExecutor<'e>, id: Uuid, i: &ChequeInput) -> DbResult<()> {
    sqlx::query(
        "UPDATE cheques SET cheque_number = $2, bank_name = $3, amount_minor = $4, due_date = $5, notes = $6,
                updated_at = now()
          WHERE id = $1",
    )
    .bind(id)
    .bind(&i.cheque_number)
    .bind(&i.bank_name)
    .bind(i.amount_minor)
    .bind(i.due_date)
    .bind(&i.notes)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn set_status<'e>(ex: impl PgExecutor<'e>, id: Uuid, status: &str) -> DbResult<()> {
    sqlx::query(
        "UPDATE cheques SET status = $2, status_changed_at = now(), updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn delete<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<()> {
    sqlx::query("DELETE FROM cheques WHERE id = $1")
        .bind(id)
        .execute(ex)
        .await
        .map(|_| ())
}

/// Drops the contract's cheques that are still PENDING (before generating a fresh set).
pub async fn delete_pending<'e>(ex: impl PgExecutor<'e>, contract_id: Uuid) -> DbResult<u64> {
    sqlx::query("DELETE FROM cheques WHERE contract_id = $1 AND status = 'PENDING'")
        .bind(contract_id)
        .execute(ex)
        .await
        .map(|r| r.rows_affected())
}

/// Renumbers a contract's cheques 1..n by due date after deletions.
pub async fn renumber<'e>(ex: impl PgExecutor<'e>, contract_id: Uuid) -> DbResult<()> {
    sqlx::query(
        "UPDATE cheques q SET seq = n.rn
           FROM (SELECT id, row_number() OVER (ORDER BY due_date, created_at) AS rn
                   FROM cheques WHERE contract_id = $1) n
          WHERE q.id = n.id AND q.seq <> n.rn",
    )
    .bind(contract_id)
    .execute(ex)
    .await
    .map(|_| ())
}

// ---------------------------------------------------------------- reminders & summary

/// PENDING cheques whose date is `on` (used for "N days before" and "today" reminders).
pub async fn pending_due_on<'e>(
    ex: impl PgExecutor<'e>,
    on: NaiveDate,
) -> DbResult<Vec<ChequeRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE q.status = 'PENDING' AND q.due_date = $1 ORDER BY q.due_date, t.name"
    ))
    .bind(on)
    .fetch_all(ex)
    .await
}

/// PENDING cheques whose date has passed.
pub async fn overdue<'e>(ex: impl PgExecutor<'e>) -> DbResult<Vec<ChequeRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE q.status = 'PENDING' AND q.due_date < CURRENT_DATE ORDER BY q.due_date, t.name"
    ))
    .fetch_all(ex)
    .await
}

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct ChequeSummaryRow {
    pub overdue_count: i64,
    pub overdue_minor: i64,
    pub due_7_count: i64,
    pub due_7_minor: i64,
    pub due_30_count: i64,
    pub due_30_minor: i64,
    pub pending_count: i64,
    pub pending_minor: i64,
    pub bounced_count: i64,
}

/// Counts and totals of pending cheques: overdue, due within 7 / 30 days, all pending.
pub async fn summary<'e>(ex: impl PgExecutor<'e>) -> DbResult<ChequeSummaryRow> {
    sqlx::query_as(
        "SELECT count(*) FILTER (WHERE status = 'PENDING' AND due_date < CURRENT_DATE) AS overdue_count,
                COALESCE(sum(amount_minor) FILTER (WHERE status = 'PENDING' AND due_date < CURRENT_DATE), 0)::bigint AS overdue_minor,
                count(*) FILTER (WHERE status = 'PENDING' AND due_date >= CURRENT_DATE AND due_date < CURRENT_DATE + 7) AS due_7_count,
                COALESCE(sum(amount_minor) FILTER (WHERE status = 'PENDING' AND due_date >= CURRENT_DATE AND due_date < CURRENT_DATE + 7), 0)::bigint AS due_7_minor,
                count(*) FILTER (WHERE status = 'PENDING' AND due_date >= CURRENT_DATE AND due_date < CURRENT_DATE + 30) AS due_30_count,
                COALESCE(sum(amount_minor) FILTER (WHERE status = 'PENDING' AND due_date >= CURRENT_DATE AND due_date < CURRENT_DATE + 30), 0)::bigint AS due_30_minor,
                count(*) FILTER (WHERE status = 'PENDING') AS pending_count,
                COALESCE(sum(amount_minor) FILTER (WHERE status = 'PENDING'), 0)::bigint AS pending_minor,
                count(*) FILTER (WHERE status = 'BOUNCED') AS bounced_count
           FROM cheques",
    )
    .fetch_one(ex)
    .await
}
