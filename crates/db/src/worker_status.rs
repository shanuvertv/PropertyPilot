//! Heartbeat row written by `renewal-worker` and read by the desktop Settings screen.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgExecutor};

use crate::DbResult;

#[derive(Debug, Clone, FromRow)]
pub struct WorkerStatusRow {
    pub version: String,
    pub hostname: String,
    pub last_heartbeat_at: DateTime<Utc>,
    pub last_sweep_at: Option<DateTime<Utc>>,
    pub last_sweep_summary: Option<Value>,
}

pub async fn heartbeat<'e>(ex: impl PgExecutor<'e>, version: &str, hostname: &str) -> DbResult<()> {
    sqlx::query(
        "INSERT INTO worker_status (id, version, hostname, last_heartbeat_at) VALUES (TRUE, $1, $2, now())
         ON CONFLICT (id) DO UPDATE SET version = EXCLUDED.version, hostname = EXCLUDED.hostname, last_heartbeat_at = now()",
    )
    .bind(version)
    .bind(hostname)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn record_sweep<'e>(ex: impl PgExecutor<'e>, summary: &Value) -> DbResult<()> {
    sqlx::query(
        "UPDATE worker_status SET last_sweep_at = now(), last_sweep_summary = $1 WHERE id = TRUE",
    )
    .bind(summary)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn get<'e>(ex: impl PgExecutor<'e>) -> DbResult<Option<WorkerStatusRow>> {
    sqlx::query_as(
        "SELECT version, hostname, last_heartbeat_at, last_sweep_at, last_sweep_summary FROM worker_status WHERE id = TRUE",
    )
    .fetch_optional(ex)
    .await
}
