use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgConnection, PgExecutor, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::paging::{ListQuery, PageResult};
use crate::DbResult;

/// A contract joined to tenant, building, units, derived expiry and any open renewal case.
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct ContractRow {
    pub id: Uuid,
    pub contract_number: String,
    pub tenant_id: Uuid,
    pub tenant_name: String,
    pub tenant_contact: Option<String>,
    pub tenant_email: Option<String>,
    pub building_id: Uuid,
    pub building_name: String,
    pub building_code: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub rent_terms: Option<String>,
    pub status: String,
    pub assigned_employee_id: Option<Uuid>,
    pub assigned_employee_name: Option<String>,
    pub previous_contract_id: Option<Uuid>,
    pub root_contract_id: Option<Uuid>,
    pub renewal_sequence: i32,
    pub notes: Option<String>,
    pub activated_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remaining_days: i32,
    pub band: String,
    pub expiring_soon: bool,
    pub urgent: bool,
    pub renewal_in_progress: bool,
    pub case_id: Option<Uuid>,
    pub renewal_status: Option<String>,
    pub notice_status: Option<String>,
    pub case_assigned_employee_id: Option<Uuid>,
    pub case_assigned_employee_name: Option<String>,
    pub unit_ids: Vec<Uuid>,
    pub unit_numbers: String,
}

#[derive(Debug, Clone, Default)]
pub struct ContractInput {
    pub contract_number: String,
    pub tenant_id: Uuid,
    pub building_id: Uuid,
    pub unit_ids: Vec<Uuid>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub rent_terms: Option<String>,
    pub assigned_employee_id: Option<Uuid>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ContractFilter {
    pub status: Option<Vec<String>>,
    pub band: Option<String>,
    pub tenant_id: Option<Uuid>,
    pub building_id: Option<Uuid>,
    pub assigned_employee_id: Option<Uuid>,
    pub expiring_soon: Option<bool>,
    pub urgent: Option<bool>,
    pub has_open_case: Option<bool>,
    /// Only contracts whose remaining days are at most this (ACTIVE ones).
    pub max_remaining_days: Option<i32>,
    pub renewal_status: Option<Vec<String>>,
    pub notice_status: Option<Vec<String>>,
    /// Match either the contract's or the open case's assigned employee.
    pub involves_employee_id: Option<Uuid>,
}

pub const SELECT: &str = "SELECT c.id, c.contract_number, c.tenant_id, t.name AS tenant_name, t.contact_person AS tenant_contact, t.email AS tenant_email,
       c.building_id, b.name AS building_name, b.code AS building_code,
       c.start_date, c.end_date, c.rent_terms, c.status, c.assigned_employee_id, emp.name AS assigned_employee_name,
       c.previous_contract_id, c.root_contract_id, c.renewal_sequence, c.notes, c.activated_at, c.ended_at,
       c.created_at, c.updated_at,
       e.remaining_days, e.band, e.expiring_soon, e.urgent, e.renewal_in_progress,
       rc.id AS case_id, rc.status AS renewal_status, rc.notice_status,
       rc.assigned_employee_id AS case_assigned_employee_id, cemp.name AS case_assigned_employee_name,
       (SELECT COALESCE(array_agg(cu.unit_id ORDER BY u.unit_number), ARRAY[]::uuid[])
          FROM contract_units cu JOIN units u ON u.id = cu.unit_id WHERE cu.contract_id = c.id) AS unit_ids,
       (SELECT COALESCE(string_agg(u.unit_number, ', ' ORDER BY u.unit_number), '')
          FROM contract_units cu JOIN units u ON u.id = cu.unit_id WHERE cu.contract_id = c.id) AS unit_numbers
  FROM contracts c
  JOIN tenants t ON t.id = c.tenant_id
  JOIN buildings b ON b.id = c.building_id
  LEFT JOIN users emp ON emp.id = c.assigned_employee_id
  JOIN v_contract_expiry e ON e.contract_id = c.id
  LEFT JOIN renewal_cases rc ON rc.contract_id = c.id AND rc.status NOT IN ('RENEWAL_COMPLETED', 'CLOSED')
  LEFT JOIN users cemp ON cemp.id = rc.assigned_employee_id";

const SORTS: &[(&str, &str)] = &[
    ("end_date", "c.end_date"),
    ("remaining_days", "e.remaining_days"),
    ("contract_number", "c.contract_number"),
    ("tenant_name", "t.name"),
    ("building_name", "b.name"),
    ("start_date", "c.start_date"),
    ("status", "c.status"),
    ("created_at", "c.created_at"),
];

fn push_filters(qb: &mut QueryBuilder<Postgres>, f: &ContractFilter, like: &Option<String>) {
    qb.push(" WHERE TRUE");
    if let Some(statuses) = &f.status {
        qb.push(" AND c.status = ANY(")
            .push_bind(statuses.clone())
            .push(")");
    }
    if let Some(band) = &f.band {
        qb.push(" AND e.band = ").push_bind(band.clone());
    }
    if let Some(t) = f.tenant_id {
        qb.push(" AND c.tenant_id = ").push_bind(t);
    }
    if let Some(b) = f.building_id {
        qb.push(" AND c.building_id = ").push_bind(b);
    }
    if let Some(a) = f.assigned_employee_id {
        qb.push(" AND c.assigned_employee_id = ").push_bind(a);
    }
    if let Some(x) = f.expiring_soon {
        qb.push(" AND e.expiring_soon = ").push_bind(x);
    }
    if let Some(x) = f.urgent {
        qb.push(" AND e.urgent = ").push_bind(x);
    }
    if let Some(x) = f.has_open_case {
        qb.push(if x {
            " AND rc.id IS NOT NULL"
        } else {
            " AND rc.id IS NULL"
        });
    }
    if let Some(d) = f.max_remaining_days {
        qb.push(" AND e.remaining_days <= ").push_bind(d);
    }
    if let Some(rs) = &f.renewal_status {
        qb.push(" AND rc.status = ANY(")
            .push_bind(rs.clone())
            .push(")");
    }
    if let Some(ns) = &f.notice_status {
        qb.push(" AND rc.notice_status = ANY(")
            .push_bind(ns.clone())
            .push(")");
    }
    if let Some(u) = f.involves_employee_id {
        qb.push(" AND (c.assigned_employee_id = ")
            .push_bind(u)
            .push(" OR rc.assigned_employee_id = ")
            .push_bind(u)
            .push(")");
    }
    if let Some(p) = like {
        qb.push(" AND (c.contract_number ILIKE ")
            .push_bind(p.clone())
            .push(" OR t.name ILIKE ")
            .push_bind(p.clone())
            .push(" OR b.name ILIKE ")
            .push_bind(p.clone())
            .push(")");
    }
}

pub async fn list(
    pool: &PgPool,
    f: &ContractFilter,
    q: &ListQuery,
) -> DbResult<PageResult<ContractRow>> {
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
        .map(ContractRow::from_row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PageResult { items, total })
}

pub async fn find<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<Option<ContractRow>> {
    sqlx::query_as(&format!("{SELECT} WHERE c.id = $1"))
        .bind(id)
        .fetch_optional(ex)
        .await
}

/// Original → Renewal 1 → Renewal 2 … for the chain that contains `id`.
pub async fn chain<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<Vec<ContractRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE COALESCE(c.root_contract_id, c.id) = (SELECT COALESCE(root_contract_id, id) FROM contracts WHERE id = $1)
         ORDER BY c.renewal_sequence"
    ))
    .bind(id)
    .fetch_all(ex)
    .await
}

pub async fn for_tenant<'e>(
    ex: impl PgExecutor<'e>,
    tenant_id: Uuid,
) -> DbResult<Vec<ContractRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE c.tenant_id = $1 ORDER BY (c.status = 'ACTIVE') DESC, c.end_date DESC"
    ))
    .bind(tenant_id)
    .fetch_all(ex)
    .await
}

pub async fn number_exists<'e>(
    ex: impl PgExecutor<'e>,
    number: &str,
    except: Option<Uuid>,
) -> DbResult<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM contracts WHERE lower(contract_number) = lower($1) AND ($2::uuid IS NULL OR id <> $2))",
    )
    .bind(number)
    .bind(except)
    .fetch_one(ex)
    .await
}

/// Next free number in the `C-0001` series (suggestion only; users may type their own).
pub async fn suggest_number<'e>(ex: impl PgExecutor<'e>) -> DbResult<String> {
    let n: i64 = sqlx::query_scalar("SELECT count(*) + 1 FROM contracts")
        .fetch_one(ex)
        .await?;
    Ok(format!("C-{n:04}"))
}

pub struct InsertContract<'a> {
    pub input: &'a ContractInput,
    pub status: &'a str,
    pub previous_contract_id: Option<Uuid>,
    pub root_contract_id: Option<Uuid>,
    pub renewal_sequence: i32,
    pub created_by: Uuid,
}

pub async fn insert(conn: &mut PgConnection, c: &InsertContract<'_>) -> DbResult<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO contracts (contract_number, tenant_id, building_id, start_date, end_date, rent_terms, status,
                                assigned_employee_id, previous_contract_id, root_contract_id, renewal_sequence, notes,
                                activated_at, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, CASE WHEN $7 = 'ACTIVE' THEN now() END, $13)
         RETURNING id",
    )
    .bind(&c.input.contract_number)
    .bind(c.input.tenant_id)
    .bind(c.input.building_id)
    .bind(c.input.start_date)
    .bind(c.input.end_date)
    .bind(&c.input.rent_terms)
    .bind(c.status)
    .bind(c.input.assigned_employee_id)
    .bind(c.previous_contract_id)
    .bind(c.root_contract_id)
    .bind(c.renewal_sequence)
    .bind(&c.input.notes)
    .bind(c.created_by)
    .fetch_one(&mut *conn)
    .await?;
    set_units(conn, id, &c.input.unit_ids).await?;
    Ok(id)
}

pub async fn set_units(
    conn: &mut PgConnection,
    contract_id: Uuid,
    unit_ids: &[Uuid],
) -> DbResult<()> {
    sqlx::query("DELETE FROM contract_units WHERE contract_id = $1")
        .bind(contract_id)
        .execute(&mut *conn)
        .await?;
    if !unit_ids.is_empty() {
        sqlx::query(
            "INSERT INTO contract_units (contract_id, unit_id) SELECT $1, unnest($2::uuid[])",
        )
        .bind(contract_id)
        .bind(unit_ids)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

pub async fn update(conn: &mut PgConnection, id: Uuid, input: &ContractInput) -> DbResult<()> {
    sqlx::query(
        "UPDATE contracts SET contract_number = $2, tenant_id = $3, building_id = $4, start_date = $5, end_date = $6,
                rent_terms = $7, assigned_employee_id = $8, notes = $9, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(&input.contract_number)
    .bind(input.tenant_id)
    .bind(input.building_id)
    .bind(input.start_date)
    .bind(input.end_date)
    .bind(&input.rent_terms)
    .bind(input.assigned_employee_id)
    .bind(&input.notes)
    .execute(&mut *conn)
    .await?;
    set_units(conn, id, &input.unit_ids).await
}

pub async fn set_status<'e>(ex: impl PgExecutor<'e>, id: Uuid, status: &str) -> DbResult<()> {
    sqlx::query(
        "UPDATE contracts SET status = $2, updated_at = now(),
                activated_at = CASE WHEN $2 = 'ACTIVE' THEN COALESCE(activated_at, now()) ELSE activated_at END,
                ended_at = CASE WHEN $2 IN ('RENEWED', 'EXPIRED', 'TERMINATED') THEN now() ELSE ended_at END
          WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn set_assigned<'e>(
    ex: impl PgExecutor<'e>,
    id: Uuid,
    employee: Option<Uuid>,
) -> DbResult<()> {
    sqlx::query("UPDATE contracts SET assigned_employee_id = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(employee)
        .execute(ex)
        .await
        .map(|_| ())
}

/// Ids of ACTIVE contracts whose end date has passed — the expiry sweep marks these EXPIRED.
pub async fn active_past_end<'e>(ex: impl PgExecutor<'e>) -> DbResult<Vec<Uuid>> {
    sqlx::query_scalar(
        "SELECT id FROM contracts WHERE status = 'ACTIVE' AND end_date < CURRENT_DATE",
    )
    .fetch_all(ex)
    .await
}

pub async fn unit_ids<'e>(ex: impl PgExecutor<'e>, contract_id: Uuid) -> DbResult<Vec<Uuid>> {
    sqlx::query_scalar("SELECT unit_id FROM contract_units WHERE contract_id = $1")
        .bind(contract_id)
        .fetch_all(ex)
        .await
}

/// Count of contracts per expiry band among ACTIVE contracts (Expired band included).
#[derive(Debug, Clone, FromRow)]
pub struct BandCount {
    pub band: String,
    pub count: i64,
}

pub async fn band_counts<'e>(ex: impl PgExecutor<'e>) -> DbResult<Vec<BandCount>> {
    sqlx::query_as(
        "SELECT e.band, count(*) AS count FROM contracts c JOIN v_contract_expiry e ON e.contract_id = c.id
          WHERE c.status = 'ACTIVE' GROUP BY e.band",
    )
    .fetch_all(ex)
    .await
}
