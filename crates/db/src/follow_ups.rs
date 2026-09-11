//! Follow-up tasks on renewal cases (spec §12).

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgExecutor, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::paging::{ListQuery, PageResult};
use crate::DbResult;

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct FollowUpRow {
    pub id: Uuid,
    pub case_id: Uuid,
    pub due_date: NaiveDate,
    pub follow_up_type: String,
    pub assigned_employee_id: Option<Uuid>,
    pub assigned_employee_name: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub contract_id: Uuid,
    pub contract_number: String,
    pub tenant_id: Uuid,
    pub tenant_name: String,
    pub building_name: String,
    pub unit_numbers: String,
    /// Days until due (negative when overdue), relative to the org's current date.
    pub days_until_due: i32,
}

const SELECT: &str = "SELECT f.id, f.case_id, f.due_date, f.follow_up_type, f.assigned_employee_id, emp.name AS assigned_employee_name,
       f.notes, f.status, f.completed_at, f.created_at,
       c.id AS contract_id, c.contract_number, c.tenant_id, t.name AS tenant_name, b.name AS building_name,
       (SELECT COALESCE(string_agg(u.unit_number, ', ' ORDER BY u.unit_number), '')
          FROM contract_units cu JOIN units u ON u.id = cu.unit_id WHERE cu.contract_id = c.id) AS unit_numbers,
       (f.due_date - CURRENT_DATE)::int AS days_until_due
  FROM follow_ups f
  JOIN renewal_cases rc ON rc.id = f.case_id
  JOIN contracts c ON c.id = rc.contract_id
  JOIN tenants t ON t.id = c.tenant_id
  JOIN buildings b ON b.id = c.building_id
  LEFT JOIN users emp ON emp.id = f.assigned_employee_id";

const SORTS: &[(&str, &str)] = &[
    ("due_date", "f.due_date, f.created_at"),
    ("tenant_name", "t.name"),
    ("status", "f.status"),
    ("created_at", "f.created_at"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Today,
    Overdue,
    Upcoming,
    Open,
    All,
}

impl Scope {
    pub fn parse(s: Option<&str>) -> Scope {
        match s {
            Some("today") => Scope::Today,
            Some("overdue") => Scope::Overdue,
            Some("upcoming") => Scope::Upcoming,
            Some("all") => Scope::All,
            _ => Scope::Open,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FollowUpFilter {
    pub scope: Option<Scope>,
    pub case_id: Option<Uuid>,
    pub assigned_employee_id: Option<Uuid>,
}

pub async fn list(
    pool: &PgPool,
    f: &FollowUpFilter,
    q: &ListQuery,
) -> DbResult<PageResult<FollowUpRow>> {
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(SELECT.replacen(
        "SELECT ",
        "SELECT count(*) OVER () AS total_count, ",
        1,
    ));
    qb.push(" WHERE TRUE");
    match f.scope.unwrap_or(Scope::Open) {
        Scope::Today => qb.push(" AND f.status = 'OPEN' AND f.due_date = CURRENT_DATE"),
        Scope::Overdue => qb.push(" AND f.status = 'OPEN' AND f.due_date < CURRENT_DATE"),
        Scope::Upcoming => qb.push(" AND f.status = 'OPEN' AND f.due_date > CURRENT_DATE"),
        Scope::Open => qb.push(" AND f.status = 'OPEN'"),
        Scope::All => qb.push(""),
    };
    if let Some(c) = f.case_id {
        qb.push(" AND f.case_id = ").push_bind(c);
    }
    if let Some(a) = f.assigned_employee_id {
        qb.push(" AND f.assigned_employee_id = ").push_bind(a);
    }
    if let Some(p) = q.like() {
        qb.push(" AND (t.name ILIKE ")
            .push_bind(p.clone())
            .push(" OR c.contract_number ILIKE ")
            .push_bind(p)
            .push(")");
    }
    qb.push(q.order_by(SORTS))
        .push(" LIMIT ")
        .push_bind(q.page_size)
        .push(" OFFSET ")
        .push_bind(q.offset());
    let rows = qb.build().fetch_all(pool).await?;
    let total: i64 = rows.first().map(|r| r.get("total_count")).unwrap_or(0);
    let items = rows
        .iter()
        .map(FollowUpRow::from_row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PageResult { items, total })
}

pub async fn find<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<Option<FollowUpRow>> {
    sqlx::query_as(&format!("{SELECT} WHERE f.id = $1"))
        .bind(id)
        .fetch_optional(ex)
        .await
}

pub async fn for_case<'e>(ex: impl PgExecutor<'e>, case_id: Uuid) -> DbResult<Vec<FollowUpRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE f.case_id = $1 ORDER BY (f.status = 'OPEN') DESC, f.due_date, f.created_at"
    ))
    .bind(case_id)
    .fetch_all(ex)
    .await
}

pub struct NewFollowUp<'a> {
    pub case_id: Uuid,
    pub due_date: NaiveDate,
    pub follow_up_type: &'a str,
    pub assigned_employee_id: Option<Uuid>,
    pub notes: Option<&'a str>,
    pub created_by: Uuid,
}

pub async fn insert<'e>(ex: impl PgExecutor<'e>, n: &NewFollowUp<'_>) -> DbResult<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO follow_ups (case_id, due_date, follow_up_type, assigned_employee_id, notes, created_by)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(n.case_id)
    .bind(n.due_date)
    .bind(n.follow_up_type)
    .bind(n.assigned_employee_id)
    .bind(n.notes)
    .bind(n.created_by)
    .fetch_one(ex)
    .await
}

pub async fn update<'e>(
    ex: impl PgExecutor<'e>,
    id: Uuid,
    due_date: NaiveDate,
    follow_up_type: &str,
    assigned_employee_id: Option<Uuid>,
    notes: Option<&str>,
) -> DbResult<()> {
    sqlx::query(
        "UPDATE follow_ups SET due_date = $2, follow_up_type = $3, assigned_employee_id = $4, notes = $5, updated_at = now()
          WHERE id = $1",
    )
    .bind(id)
    .bind(due_date)
    .bind(follow_up_type)
    .bind(assigned_employee_id)
    .bind(notes)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn set_status<'e>(
    ex: impl PgExecutor<'e>,
    id: Uuid,
    status: &str,
    by: Uuid,
) -> DbResult<()> {
    sqlx::query(
        "UPDATE follow_ups SET status = $2, updated_at = now(),
                completed_at = CASE WHEN $2 = 'DONE' THEN now() END,
                completed_by = CASE WHEN $2 = 'DONE' THEN $3 END
          WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .bind(by)
    .execute(ex)
    .await
    .map(|_| ())
}

#[derive(Debug, Clone, FromRow)]
pub struct FollowUpCounts {
    pub today: i64,
    pub overdue: i64,
    pub upcoming: i64,
}

pub async fn counts<'e>(
    ex: impl PgExecutor<'e>,
    assigned_employee_id: Option<Uuid>,
) -> DbResult<FollowUpCounts> {
    sqlx::query_as(
        "SELECT count(*) FILTER (WHERE due_date = CURRENT_DATE) AS today,
                count(*) FILTER (WHERE due_date < CURRENT_DATE) AS overdue,
                count(*) FILTER (WHERE due_date > CURRENT_DATE) AS upcoming
           FROM follow_ups WHERE status = 'OPEN' AND ($1::uuid IS NULL OR assigned_employee_id = $1)",
    )
    .bind(assigned_employee_id)
    .fetch_one(ex)
    .await
}
