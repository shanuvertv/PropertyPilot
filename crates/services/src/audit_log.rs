//! Small helper so every service writes audit rows the same way.

use renewal_db::audit::{self, AuditEntry};
use serde::Serialize;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::error::ServiceResult;
use crate::session::Session;

pub async fn log<'e>(
    ex: impl PgExecutor<'e>,
    actor: &Session,
    entity_type: &str,
    entity_id: Uuid,
    action: &str,
    before: Option<&impl Serialize>,
    after: Option<&impl Serialize>,
) -> ServiceResult<()> {
    let before = before.map(serde_json::to_value).transpose().unwrap_or(None);
    let after = after.map(serde_json::to_value).transpose().unwrap_or(None);
    audit::record(
        ex,
        &AuditEntry {
            actor_id: Some(actor.user_id),
            entity_type,
            entity_id: Some(entity_id),
            action,
            before,
            after,
        },
    )
    .await?;
    Ok(())
}

/// Marker for "no snapshot" so callers can write `NONE` instead of `None::<&()>`.
pub const NONE: Option<&()> = None;
