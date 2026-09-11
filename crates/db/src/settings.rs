//! Key/value settings (org profile, timezone, expiry thresholds, mailbox, letterhead).

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::DbResult;

pub const ORG_NAME: &str = "org.name";
pub const ORG_TIMEZONE: &str = "org.timezone";
pub const EXPIRY_THRESHOLDS: &str = "expiry.thresholds";

pub async fn get_raw<'e>(ex: impl PgExecutor<'e>, key: &str) -> DbResult<Option<Value>> {
    sqlx::query_scalar("SELECT value FROM settings WHERE key = $1")
        .bind(key)
        .fetch_optional(ex)
        .await
}

/// Typed read; a missing key or a value that fails to parse yields `None`.
pub async fn get<'e, T: DeserializeOwned>(
    ex: impl PgExecutor<'e>,
    key: &str,
) -> DbResult<Option<T>> {
    Ok(get_raw(ex, key)
        .await?
        .and_then(|v| serde_json::from_value(v).ok()))
}

pub async fn set<'e, T: Serialize>(
    ex: impl PgExecutor<'e>,
    key: &str,
    value: &T,
    updated_by: Option<Uuid>,
) -> DbResult<()> {
    let json = serde_json::to_value(value).map_err(|e| sqlx::Error::Encode(Box::new(e)))?;
    sqlx::query(
        "INSERT INTO settings (key, value, updated_by) VALUES ($1, $2, $3)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = now(), updated_by = EXCLUDED.updated_by",
    )
    .bind(key)
    .bind(json)
    .bind(updated_by)
    .execute(ex)
    .await
    .map(|_| ())
}
