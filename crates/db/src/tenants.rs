use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgExecutor, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::paging::{ListQuery, PageResult};
use crate::DbResult;

/// Tenant master record with live counts (spec §4).
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct TenantRow {
    pub id: Uuid,
    pub name: String,
    pub contact_person: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub alt_contact: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub active_contracts: i64,
    pub current_units: i64,
}

#[derive(Debug, Clone, Default)]
pub struct TenantInput {
    pub name: String,
    pub contact_person: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub alt_contact: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
}

const SELECT: &str = "SELECT t.id, t.name, t.contact_person, t.mobile, t.email, t.alt_contact, t.address, t.notes,
       t.created_at, t.updated_at, t.archived_at,
       (SELECT count(*) FROM contracts c WHERE c.tenant_id = t.id AND c.status = 'ACTIVE') AS active_contracts,
       (SELECT count(*) FROM contracts c JOIN contract_units cu ON cu.contract_id = c.id
         WHERE c.tenant_id = t.id AND c.status = 'ACTIVE') AS current_units
  FROM tenants t";

const SORTS: &[(&str, &str)] = &[
    ("name", "t.name"),
    ("contact_person", "t.contact_person"),
    ("email", "t.email"),
    ("mobile", "t.mobile"),
    ("active_contracts", "active_contracts"),
    ("created_at", "t.created_at"),
];

#[derive(Debug, Clone, Default)]
pub struct TenantFilter {
    /// Tenants holding an active contract in this building.
    pub building_id: Option<Uuid>,
    /// `Some(true)` = with an active contract, `Some(false)` = without one.
    pub active: Option<bool>,
}

pub async fn list(
    pool: &PgPool,
    f: &TenantFilter,
    q: &ListQuery,
) -> DbResult<PageResult<TenantRow>> {
    let like = q.like();
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(SELECT.replacen(
        "SELECT ",
        "SELECT count(*) OVER () AS total_count, ",
        1,
    ));
    qb.push(" WHERE t.archived_at IS NULL");
    if let Some(b) = f.building_id {
        qb.push(" AND EXISTS (SELECT 1 FROM contracts c WHERE c.tenant_id = t.id AND c.status = 'ACTIVE' AND c.building_id = ")
            .push_bind(b)
            .push(")");
    }
    if let Some(active) = f.active {
        qb.push(if active {
            " AND EXISTS"
        } else {
            " AND NOT EXISTS"
        })
        .push(" (SELECT 1 FROM contracts c WHERE c.tenant_id = t.id AND c.status = 'ACTIVE')");
    }
    if let Some(p) = &like {
        qb.push(" AND (t.name ILIKE ")
            .push_bind(p.clone())
            .push(" OR t.contact_person ILIKE ")
            .push_bind(p.clone())
            .push(" OR t.email ILIKE ")
            .push_bind(p.clone())
            .push(" OR t.mobile ILIKE ")
            .push_bind(p.clone())
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
        .map(TenantRow::from_row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PageResult { items, total })
}

pub async fn all_active<'e>(ex: impl PgExecutor<'e>) -> DbResult<Vec<TenantRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE t.archived_at IS NULL ORDER BY t.name"
    ))
    .fetch_all(ex)
    .await
}

pub async fn find<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<Option<TenantRow>> {
    sqlx::query_as(&format!("{SELECT} WHERE t.id = $1"))
        .bind(id)
        .fetch_optional(ex)
        .await
}

pub async fn insert<'e>(
    ex: impl PgExecutor<'e>,
    input: &TenantInput,
    created_by: Uuid,
) -> DbResult<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO tenants (name, contact_person, mobile, email, alt_contact, address, notes, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
    )
    .bind(&input.name)
    .bind(&input.contact_person)
    .bind(&input.mobile)
    .bind(&input.email)
    .bind(&input.alt_contact)
    .bind(&input.address)
    .bind(&input.notes)
    .bind(created_by)
    .fetch_one(ex)
    .await
}

pub async fn update<'e>(ex: impl PgExecutor<'e>, id: Uuid, input: &TenantInput) -> DbResult<()> {
    sqlx::query(
        "UPDATE tenants SET name = $2, contact_person = $3, mobile = $4, email = $5, alt_contact = $6, address = $7,
                notes = $8, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(&input.name)
    .bind(&input.contact_person)
    .bind(&input.mobile)
    .bind(&input.email)
    .bind(&input.alt_contact)
    .bind(&input.address)
    .bind(&input.notes)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn archive<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<()> {
    sqlx::query("UPDATE tenants SET archived_at = now(), updated_at = now() WHERE id = $1 AND archived_at IS NULL")
        .bind(id)
        .execute(ex)
        .await
        .map(|_| ())
}
