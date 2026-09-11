use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgExecutor};
use uuid::Uuid;

use crate::DbResult;

#[derive(Debug, Clone, FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    /// Stored as SCREAMING_SNAKE_CASE text; parse with `renewal_core::Role`.
    pub role: String,
    pub password_hash: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewUser<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub role: &'a str,
    pub password_hash: &'a str,
}

const COLS: &str =
    "id, name, email, role, password_hash, active, created_at, updated_at, last_login_at";

pub async fn count<'e>(ex: impl PgExecutor<'e>) -> DbResult<i64> {
    sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(ex)
        .await
}

pub async fn find_by_email<'e>(ex: impl PgExecutor<'e>, email: &str) -> DbResult<Option<UserRow>> {
    sqlx::query_as(&format!(
        "SELECT {COLS} FROM users WHERE lower(email) = lower($1)"
    ))
    .bind(email)
    .fetch_optional(ex)
    .await
}

pub async fn find_by_id<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<Option<UserRow>> {
    sqlx::query_as(&format!("SELECT {COLS} FROM users WHERE id = $1"))
        .bind(id)
        .fetch_optional(ex)
        .await
}

pub async fn list<'e>(ex: impl PgExecutor<'e>) -> DbResult<Vec<UserRow>> {
    sqlx::query_as(&format!(
        "SELECT {COLS} FROM users ORDER BY active DESC, name"
    ))
    .fetch_all(ex)
    .await
}

pub async fn insert<'e>(ex: impl PgExecutor<'e>, user: &NewUser<'_>) -> DbResult<UserRow> {
    sqlx::query_as(&format!(
        "INSERT INTO users (name, email, role, password_hash) VALUES ($1, $2, $3, $4) RETURNING {COLS}"
    ))
    .bind(user.name)
    .bind(user.email)
    .bind(user.role)
    .bind(user.password_hash)
    .fetch_one(ex)
    .await
}

pub async fn touch_last_login<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<()> {
    sqlx::query("UPDATE users SET last_login_at = now() WHERE id = $1")
        .bind(id)
        .execute(ex)
        .await
        .map(|_| ())
}

pub async fn set_active<'e>(ex: impl PgExecutor<'e>, id: Uuid, active: bool) -> DbResult<()> {
    sqlx::query("UPDATE users SET active = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(active)
        .execute(ex)
        .await
        .map(|_| ())
}

pub async fn set_password_hash<'e>(ex: impl PgExecutor<'e>, id: Uuid, hash: &str) -> DbResult<()> {
    sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(hash)
        .execute(ex)
        .await
        .map(|_| ())
}
