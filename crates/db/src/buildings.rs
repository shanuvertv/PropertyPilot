use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection, PgExecutor, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::paging::{ListQuery, PageResult};
use crate::DbResult;

/// A building with derived unit counts (spec §2 "Building Summary").
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct BuildingRow {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub location: Option<String>,
    pub building_type: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub total_units: i64,
    pub occupied_units: i64,
    pub vacant_units: i64,
    pub reserved_units: i64,
    pub maintenance_units: i64,
}

#[derive(Debug, Clone, Default)]
pub struct BuildingInput {
    pub name: String,
    pub code: String,
    pub location: Option<String>,
    pub building_type: Option<String>,
    pub notes: Option<String>,
}

const SELECT: &str = "SELECT b.id, b.name, b.code, b.location, b.building_type, b.notes, b.created_at, b.updated_at, b.archived_at,
       count(u.id) FILTER (WHERE u.archived_at IS NULL) AS total_units,
       count(u.id) FILTER (WHERE u.archived_at IS NULL AND u.status = 'OCCUPIED') AS occupied_units,
       count(u.id) FILTER (WHERE u.archived_at IS NULL AND u.status = 'VACANT') AS vacant_units,
       count(u.id) FILTER (WHERE u.archived_at IS NULL AND u.status = 'RESERVED') AS reserved_units,
       count(u.id) FILTER (WHERE u.archived_at IS NULL AND u.status = 'MAINTENANCE') AS maintenance_units
  FROM buildings b LEFT JOIN units u ON u.building_id = b.id";

const SORTS: &[(&str, &str)] = &[
    ("name", "b.name"),
    ("code", "b.code"),
    ("location", "b.location"),
    ("building_type", "b.building_type"),
    ("total_units", "total_units"),
    ("vacant_units", "vacant_units"),
    ("occupied_units", "occupied_units"),
    ("created_at", "b.created_at"),
];

pub async fn list(pool: &PgPool, q: &ListQuery) -> DbResult<PageResult<BuildingRow>> {
    let like = q.like();
    let mut count: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT count(*) FROM buildings b WHERE b.archived_at IS NULL");
    if let Some(p) = &like {
        count
            .push(" AND (b.name ILIKE ")
            .push_bind(p.clone())
            .push(" OR b.code ILIKE ")
            .push_bind(p.clone())
            .push(" OR b.location ILIKE ")
            .push_bind(p.clone())
            .push(")");
    }
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;

    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(SELECT);
    qb.push(" WHERE b.archived_at IS NULL");
    if let Some(p) = &like {
        qb.push(" AND (b.name ILIKE ")
            .push_bind(p.clone())
            .push(" OR b.code ILIKE ")
            .push_bind(p.clone())
            .push(" OR b.location ILIKE ")
            .push_bind(p.clone())
            .push(")");
    }
    qb.push(" GROUP BY b.id")
        .push(q.order_by(SORTS))
        .push(" LIMIT ")
        .push_bind(q.page_size)
        .push(" OFFSET ")
        .push_bind(q.offset());
    let items = qb.build_query_as::<BuildingRow>().fetch_all(pool).await?;
    Ok(PageResult { items, total })
}

/// Every non-archived building, lightest form, for dropdowns.
pub async fn all_active<'e>(ex: impl PgExecutor<'e>) -> DbResult<Vec<BuildingRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE b.archived_at IS NULL GROUP BY b.id ORDER BY b.name"
    ))
    .fetch_all(ex)
    .await
}

pub async fn find<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<Option<BuildingRow>> {
    sqlx::query_as(&format!("{SELECT} WHERE b.id = $1 GROUP BY b.id"))
        .bind(id)
        .fetch_optional(ex)
        .await
}

pub async fn code_exists<'e>(
    ex: impl PgExecutor<'e>,
    code: &str,
    except: Option<Uuid>,
) -> DbResult<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM buildings WHERE lower(code) = lower($1) AND archived_at IS NULL AND ($2::uuid IS NULL OR id <> $2))",
    )
    .bind(code)
    .bind(except)
    .fetch_one(ex)
    .await
}

pub async fn insert<'e>(
    ex: impl PgExecutor<'e>,
    input: &BuildingInput,
    created_by: Uuid,
) -> DbResult<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO buildings (name, code, location, building_type, notes, created_by) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(&input.name)
    .bind(&input.code)
    .bind(&input.location)
    .bind(&input.building_type)
    .bind(&input.notes)
    .bind(created_by)
    .fetch_one(ex)
    .await
}

pub async fn update<'e>(ex: impl PgExecutor<'e>, id: Uuid, input: &BuildingInput) -> DbResult<()> {
    sqlx::query(
        "UPDATE buildings SET name = $2, code = $3, location = $4, building_type = $5, notes = $6, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(&input.name)
    .bind(&input.code)
    .bind(&input.location)
    .bind(&input.building_type)
    .bind(&input.notes)
    .execute(ex)
    .await
    .map(|_| ())
}

/// Archives the building and all of its units.
pub async fn archive(conn: &mut PgConnection, id: Uuid) -> DbResult<()> {
    sqlx::query("UPDATE units SET archived_at = now(), updated_at = now() WHERE building_id = $1 AND archived_at IS NULL")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    sqlx::query("UPDATE buildings SET archived_at = now(), updated_at = now() WHERE id = $1 AND archived_at IS NULL")
        .bind(id)
        .execute(&mut *conn)
        .await
        .map(|_| ())
}
