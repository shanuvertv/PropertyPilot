//! Occupants: the people living in a unit (shared accommodation), with move-in/out dates.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgExecutor};
use uuid::Uuid;

use crate::DbResult;

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct OccupantRow {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub unit_number: String,
    pub building_id: Uuid,
    pub building_name: String,
    pub tenant_id: Option<Uuid>,
    pub tenant_name: Option<String>,
    pub full_name: String,
    pub id_number: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub bed_label: Option<String>,
    pub move_in: NaiveDate,
    pub move_out: Option<NaiveDate>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct OccupantInput {
    pub tenant_id: Option<Uuid>,
    pub full_name: String,
    pub id_number: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub bed_label: Option<String>,
    pub move_in: NaiveDate,
    pub move_out: Option<NaiveDate>,
    pub notes: Option<String>,
}

const SELECT: &str =
    "SELECT o.id, o.unit_id, u.unit_number, u.building_id, b.name AS building_name,
       o.tenant_id, t.name AS tenant_name, o.full_name, o.id_number, o.phone, o.email, o.bed_label,
       o.move_in, o.move_out, o.notes, o.created_at, o.updated_at
  FROM occupants o
  JOIN units u ON u.id = o.unit_id
  JOIN buildings b ON b.id = u.building_id
  LEFT JOIN tenants t ON t.id = o.tenant_id";

/// Occupants of a unit; current ones first, then past ones (when `include_past`).
pub async fn for_unit<'e>(
    ex: impl PgExecutor<'e>,
    unit_id: Uuid,
    include_past: bool,
) -> DbResult<Vec<OccupantRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE o.unit_id = $1 AND ($2 OR o.move_out IS NULL OR o.move_out >= CURRENT_DATE)
         ORDER BY (o.move_out IS NOT NULL AND o.move_out < CURRENT_DATE), o.bed_label NULLS LAST, o.full_name"
    ))
    .bind(unit_id)
    .bind(include_past)
    .fetch_all(ex)
    .await
}

/// Occupants living in the unit on a given date (for splitting a bill dated that day).
pub async fn present_on<'e>(
    ex: impl PgExecutor<'e>,
    unit_id: Uuid,
    on: NaiveDate,
) -> DbResult<Vec<OccupantRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE o.unit_id = $1 AND o.move_in <= $2 AND (o.move_out IS NULL OR o.move_out >= $2)
         ORDER BY o.bed_label NULLS LAST, o.full_name"
    ))
    .bind(unit_id)
    .bind(on)
    .fetch_all(ex)
    .await
}

pub async fn find<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<Option<OccupantRow>> {
    sqlx::query_as(&format!("{SELECT} WHERE o.id = $1"))
        .bind(id)
        .fetch_optional(ex)
        .await
}

pub async fn insert<'e>(
    ex: impl PgExecutor<'e>,
    unit_id: Uuid,
    i: &OccupantInput,
    created_by: Uuid,
) -> DbResult<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO occupants (unit_id, tenant_id, full_name, id_number, phone, email, bed_label, move_in, move_out, notes, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
    )
    .bind(unit_id)
    .bind(i.tenant_id)
    .bind(&i.full_name)
    .bind(&i.id_number)
    .bind(&i.phone)
    .bind(&i.email)
    .bind(&i.bed_label)
    .bind(i.move_in)
    .bind(i.move_out)
    .bind(&i.notes)
    .bind(created_by)
    .fetch_one(ex)
    .await
}

pub async fn update<'e>(ex: impl PgExecutor<'e>, id: Uuid, i: &OccupantInput) -> DbResult<()> {
    sqlx::query(
        "UPDATE occupants SET tenant_id = $2, full_name = $3, id_number = $4, phone = $5, email = $6,
                bed_label = $7, move_in = $8, move_out = $9, notes = $10, updated_at = now()
          WHERE id = $1",
    )
    .bind(id)
    .bind(i.tenant_id)
    .bind(&i.full_name)
    .bind(&i.id_number)
    .bind(&i.phone)
    .bind(&i.email)
    .bind(&i.bed_label)
    .bind(i.move_in)
    .bind(i.move_out)
    .bind(&i.notes)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn set_move_out<'e>(
    ex: impl PgExecutor<'e>,
    id: Uuid,
    move_out: Option<NaiveDate>,
) -> DbResult<()> {
    sqlx::query("UPDATE occupants SET move_out = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(move_out)
        .execute(ex)
        .await
        .map(|_| ())
}

/// True when the occupant has any expense share — then they cannot be deleted, only moved out.
pub async fn has_shares<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM expense_shares WHERE occupant_id = $1)")
        .bind(id)
        .fetch_one(ex)
        .await
}

pub async fn delete<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<()> {
    sqlx::query("DELETE FROM occupants WHERE id = $1")
        .bind(id)
        .execute(ex)
        .await
        .map(|_| ())
}

pub async fn count_current<'e>(ex: impl PgExecutor<'e>, unit_id: Uuid) -> DbResult<i64> {
    sqlx::query_scalar(
        "SELECT count(*) FROM occupants WHERE unit_id = $1 AND (move_out IS NULL OR move_out >= CURRENT_DATE)",
    )
    .bind(unit_id)
    .fetch_one(ex)
    .await
}
