use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgExecutor, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::paging::{ListQuery, PageResult};
use crate::DbResult;

/// One row of the Unit-Wise Summary (spec §3 + §16): the unit joined to its current
/// ACTIVE contract, that contract's tenant, derived expiry and any open renewal case.
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct UnitSummaryRow {
    pub id: Uuid,
    pub building_id: Uuid,
    pub building_name: String,
    pub building_code: String,
    pub unit_number: String,
    pub floor: Option<String>,
    pub unit_type: Option<String>,
    pub status: String,
    /// Number of tenants on the unit's active contract (what its bills are split by); 0 when vacant.
    pub occupant_count: i32,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub contract_id: Option<Uuid>,
    pub contract_number: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    /// This unit's rent under its active contract (minor units).
    pub rent_amount_minor: Option<i64>,
    pub tenant_id: Option<Uuid>,
    pub tenant_name: Option<String>,
    pub remaining_days: Option<i32>,
    pub band: Option<String>,
    pub expiring_soon: Option<bool>,
    pub urgent: Option<bool>,
    pub case_id: Option<Uuid>,
    pub renewal_status: Option<String>,
    pub assigned_employee_id: Option<Uuid>,
    pub assigned_employee_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UnitInput {
    pub building_id: Uuid,
    pub unit_number: String,
    pub floor: Option<String>,
    pub unit_type: Option<String>,
    pub status: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UnitFilter {
    pub building_id: Option<Uuid>,
    pub status: Option<String>,
    pub band: Option<String>,
    pub renewal_status: Option<String>,
    pub tenant_id: Option<Uuid>,
    /// Only units whose current contract is expiring soon.
    pub expiring_soon: Option<bool>,
}

const SELECT: &str = "SELECT u.id, u.building_id, b.name AS building_name, b.code AS building_code, u.unit_number, u.floor,
       u.unit_type, u.status, COALESCE(c.unit_occupants, 0) AS occupant_count, u.notes, u.created_at, u.updated_at,
       c.id AS contract_id, c.contract_number, c.start_date, c.end_date, c.unit_rent AS rent_amount_minor,
       t.id AS tenant_id, t.name AS tenant_name,
       e.remaining_days, e.band, e.expiring_soon, e.urgent,
       rc.id AS case_id, rc.status AS renewal_status,
       emp.id AS assigned_employee_id, emp.name AS assigned_employee_name
  FROM units u
  JOIN buildings b ON b.id = u.building_id
  LEFT JOIN LATERAL (
        SELECT c.*, cu.occupant_count AS unit_occupants, cu.rent_amount_minor AS unit_rent
          FROM contracts c JOIN contract_units cu ON cu.contract_id = c.id
         WHERE cu.unit_id = u.id AND c.status = 'ACTIVE'
         ORDER BY c.end_date DESC LIMIT 1) c ON TRUE
  LEFT JOIN tenants t ON t.id = c.tenant_id
  LEFT JOIN v_contract_expiry e ON e.contract_id = c.id
  LEFT JOIN renewal_cases rc ON rc.contract_id = c.id AND rc.status NOT IN ('RENEWAL_COMPLETED', 'CLOSED')
  LEFT JOIN users emp ON emp.id = COALESCE(rc.assigned_employee_id, c.assigned_employee_id)";

const SORTS: &[(&str, &str)] = &[
    ("building_name", "b.name, u.unit_number"),
    ("unit_number", "u.unit_number"),
    ("floor", "u.floor"),
    ("unit_type", "u.unit_type"),
    ("status", "u.status"),
    ("tenant_name", "t.name"),
    ("start_date", "c.start_date"),
    ("end_date", "c.end_date"),
    ("remaining_days", "e.remaining_days"),
    ("renewal_status", "rc.status"),
    ("created_at", "u.created_at"),
];

fn push_filters(qb: &mut QueryBuilder<Postgres>, f: &UnitFilter, like: &Option<String>) {
    qb.push(" WHERE u.archived_at IS NULL");
    if let Some(b) = f.building_id {
        qb.push(" AND u.building_id = ").push_bind(b);
    }
    if let Some(s) = &f.status {
        qb.push(" AND u.status = ").push_bind(s.clone());
    }
    if let Some(band) = &f.band {
        qb.push(" AND e.band = ").push_bind(band.clone());
    }
    if let Some(rs) = &f.renewal_status {
        qb.push(" AND rc.status = ").push_bind(rs.clone());
    }
    if let Some(t) = f.tenant_id {
        qb.push(" AND t.id = ").push_bind(t);
    }
    if let Some(x) = f.expiring_soon {
        qb.push(" AND COALESCE(e.expiring_soon, FALSE) = ")
            .push_bind(x);
    }
    if let Some(p) = like {
        qb.push(" AND (u.unit_number ILIKE ")
            .push_bind(p.clone())
            .push(" OR b.name ILIKE ")
            .push_bind(p.clone())
            .push(" OR t.name ILIKE ")
            .push_bind(p.clone())
            .push(" OR c.contract_number ILIKE ")
            .push_bind(p.clone())
            .push(")");
    }
}

pub async fn list(
    pool: &PgPool,
    f: &UnitFilter,
    q: &ListQuery,
) -> DbResult<PageResult<UnitSummaryRow>> {
    // A window count over the filtered rows keeps the count and the page in one query,
    // so filters on tenant / expiry / renewal columns apply identically to both.
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
        .map(UnitSummaryRow::from_row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PageResult { items, total })
}

pub async fn find<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<Option<UnitSummaryRow>> {
    sqlx::query_as(&format!("{SELECT} WHERE u.id = $1"))
        .bind(id)
        .fetch_optional(ex)
        .await
}

pub async fn for_building<'e>(
    ex: impl PgExecutor<'e>,
    building_id: Uuid,
) -> DbResult<Vec<UnitSummaryRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE u.archived_at IS NULL AND u.building_id = $1 ORDER BY u.unit_number"
    ))
    .bind(building_id)
    .fetch_all(ex)
    .await
}

pub async fn find_many<'e>(ex: impl PgExecutor<'e>, ids: &[Uuid]) -> DbResult<Vec<UnitSummaryRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE u.id = ANY($1) ORDER BY u.unit_number"
    ))
    .bind(ids)
    .fetch_all(ex)
    .await
}

pub async fn number_exists<'e>(
    ex: impl PgExecutor<'e>,
    building_id: Uuid,
    unit_number: &str,
    except: Option<Uuid>,
) -> DbResult<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM units WHERE building_id = $1 AND lower(unit_number) = lower($2)
                        AND archived_at IS NULL AND ($3::uuid IS NULL OR id <> $3))",
    )
    .bind(building_id)
    .bind(unit_number)
    .bind(except)
    .fetch_one(ex)
    .await
}

pub async fn insert<'e>(
    ex: impl PgExecutor<'e>,
    input: &UnitInput,
    created_by: Uuid,
) -> DbResult<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO units (building_id, unit_number, floor, unit_type, status, notes, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(input.building_id)
    .bind(&input.unit_number)
    .bind(&input.floor)
    .bind(&input.unit_type)
    .bind(&input.status)
    .bind(&input.notes)
    .bind(created_by)
    .fetch_one(ex)
    .await
}

pub async fn update<'e>(ex: impl PgExecutor<'e>, id: Uuid, input: &UnitInput) -> DbResult<()> {
    sqlx::query(
        "UPDATE units SET building_id = $2, unit_number = $3, floor = $4, unit_type = $5, status = $6, notes = $7, updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(input.building_id)
    .bind(&input.unit_number)
    .bind(&input.floor)
    .bind(&input.unit_type)
    .bind(&input.status)
    .bind(&input.notes)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn set_status<'e>(ex: impl PgExecutor<'e>, id: Uuid, status: &str) -> DbResult<()> {
    sqlx::query("UPDATE units SET status = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(status)
        .execute(ex)
        .await
        .map(|_| ())
}

/// Bulk status change used by contract activation / termination (PLAN.md §3.6).
pub async fn set_status_many<'e>(
    ex: impl PgExecutor<'e>,
    ids: &[Uuid],
    status: &str,
    only_from: Option<&[&str]>,
) -> DbResult<u64> {
    let from: Vec<String> = only_from
        .map(|s| s.iter().map(|x| x.to_string()).collect())
        .unwrap_or_default();
    sqlx::query(
        "UPDATE units SET status = $2, updated_at = now()
          WHERE id = ANY($1) AND ($3::text[] = '{}' OR status = ANY($3))",
    )
    .bind(ids)
    .bind(status)
    .bind(&from)
    .execute(ex)
    .await
    .map(|r| r.rows_affected())
}

pub async fn archive<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<()> {
    sqlx::query("UPDATE units SET archived_at = now(), updated_at = now() WHERE id = $1 AND archived_at IS NULL")
        .bind(id)
        .execute(ex)
        .await
        .map(|_| ())
}

/// Ids of the given units that currently have an ACTIVE contract (other than `except`).
pub async fn occupied_by_other_contract<'e>(
    ex: impl PgExecutor<'e>,
    unit_ids: &[Uuid],
    except: Option<Uuid>,
) -> DbResult<Vec<Uuid>> {
    sqlx::query_scalar(
        "SELECT DISTINCT cu.unit_id FROM contract_units cu JOIN contracts c ON c.id = cu.contract_id
          WHERE cu.unit_id = ANY($1) AND c.status = 'ACTIVE' AND ($2::uuid IS NULL OR c.id <> $2)",
    )
    .bind(unit_ids)
    .bind(except)
    .fetch_all(ex)
    .await
}
