use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgExecutor};
use uuid::Uuid;

use crate::DbResult;

#[derive(Debug, Clone, FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub async fn insert<'e>(
    ex: impl PgExecutor<'e>,
    user_id: Uuid,
    token_hash: &str,
    user_agent: Option<&str>,
    expires_at: DateTime<Utc>,
) -> DbResult<SessionRow> {
    sqlx::query_as(
        "INSERT INTO sessions (user_id, token_hash, user_agent, expires_at) VALUES ($1, $2, $3, $4)
         RETURNING id, user_id, created_at, last_seen_at, expires_at",
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(user_agent)
    .bind(expires_at)
    .fetch_one(ex)
    .await
}

/// Looks up a live (unexpired, unrevoked) session and bumps `last_seen_at`.
pub async fn touch_by_token_hash<'e>(
    ex: impl PgExecutor<'e>,
    token_hash: &str,
) -> DbResult<Option<SessionRow>> {
    sqlx::query_as(
        "UPDATE sessions SET last_seen_at = now()
         WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > now()
         RETURNING id, user_id, created_at, last_seen_at, expires_at",
    )
    .bind(token_hash)
    .fetch_optional(ex)
    .await
}

pub async fn revoke<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<()> {
    sqlx::query("UPDATE sessions SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL")
        .bind(id)
        .execute(ex)
        .await
        .map(|_| ())
}

pub async fn revoke_all_for_user<'e>(ex: impl PgExecutor<'e>, user_id: Uuid) -> DbResult<u64> {
    sqlx::query("UPDATE sessions SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(user_id)
        .execute(ex)
        .await
        .map(|r| r.rows_affected())
}

/// Housekeeping for the scheduler: drop sessions expired more than 30 days ago.
pub async fn purge_expired<'e>(ex: impl PgExecutor<'e>) -> DbResult<u64> {
    sqlx::query("DELETE FROM sessions WHERE expires_at < now() - interval '30 days'")
        .execute(ex)
        .await
        .map(|r| r.rows_affected())
}

pub async fn revoke_all_except<'e>(
    ex: impl PgExecutor<'e>,
    user_id: Uuid,
    keep: Uuid,
) -> DbResult<u64> {
    sqlx::query("UPDATE sessions SET revoked_at = now() WHERE user_id = $1 AND id <> $2 AND revoked_at IS NULL")
        .bind(user_id)
        .bind(keep)
        .execute(ex)
        .await
        .map(|r| r.rows_affected())
}
