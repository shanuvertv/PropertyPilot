//! Audit trail writer (spec §19). Call inside the same transaction as the change it records.

use serde_json::Value;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::DbResult;

#[derive(Debug, Clone)]
pub struct AuditEntry<'a> {
    pub actor_id: Option<Uuid>,
    /// e.g. "user", "contract", "renewal_case"
    pub entity_type: &'a str,
    pub entity_id: Option<Uuid>,
    /// e.g. "CREATED", "UPDATED", "RENEWAL_STARTED", "NOTICE_SENT"
    pub action: &'a str,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

pub async fn record<'e>(ex: impl PgExecutor<'e>, entry: &AuditEntry<'_>) -> DbResult<()> {
    sqlx::query(
        "INSERT INTO audit_logs (actor_id, entity_type, entity_id, action, before_json, after_json)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(entry.actor_id)
    .bind(entry.entity_type)
    .bind(entry.entity_id)
    .bind(entry.action)
    .bind(&entry.before)
    .bind(&entry.after)
    .execute(ex)
    .await
    .map(|_| ())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditRow {
    pub id: i64,
    pub actor_id: Option<Uuid>,
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub action: String,
    pub before_json: Option<Value>,
    pub after_json: Option<Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Most recent entries first. `limit` is capped at 500.
pub async fn recent<'e>(ex: impl PgExecutor<'e>, limit: i64) -> DbResult<Vec<AuditRow>> {
    sqlx::query_as(
        "SELECT id, actor_id, entity_type, entity_id, action, before_json, after_json, created_at
         FROM audit_logs ORDER BY id DESC LIMIT $1",
    )
    .bind(limit.clamp(1, 500))
    .fetch_all(ex)
    .await
}

pub async fn for_entity<'e>(
    ex: impl PgExecutor<'e>,
    entity_type: &str,
    entity_id: Uuid,
) -> DbResult<Vec<AuditRow>> {
    sqlx::query_as(
        "SELECT id, actor_id, entity_type, entity_id, action, before_json, after_json, created_at
         FROM audit_logs WHERE entity_type = $1 AND entity_id = $2 ORDER BY id DESC",
    )
    .bind(entity_type)
    .bind(entity_id)
    .fetch_all(ex)
    .await
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AuditListRow {
    pub id: i64,
    pub actor_id: Option<Uuid>,
    pub actor_name: Option<String>,
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub action: String,
    pub before_json: Option<Value>,
    pub after_json: Option<Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub actor_id: Option<Uuid>,
    pub action: Option<String>,
}

/// Paged audit trail, newest first (spec section 19).
pub async fn list(
    pool: &crate::PgPool,
    f: &AuditFilter,
    q: &crate::paging::ListQuery,
) -> DbResult<crate::paging::PageResult<AuditListRow>> {
    use sqlx::Row;
    let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
        "SELECT count(*) OVER () AS total_count, a.id, a.actor_id, u.name AS actor_name, a.entity_type, a.entity_id, a.action,
                a.before_json, a.after_json, a.created_at
           FROM audit_logs a LEFT JOIN users u ON u.id = a.actor_id WHERE TRUE",
    );
    if let Some(t) = &f.entity_type {
        qb.push(" AND a.entity_type = ").push_bind(t.clone());
    }
    if let Some(id) = f.entity_id {
        qb.push(" AND a.entity_id = ").push_bind(id);
    }
    if let Some(id) = f.actor_id {
        qb.push(" AND a.actor_id = ").push_bind(id);
    }
    if let Some(a) = &f.action {
        qb.push(" AND a.action = ").push_bind(a.clone());
    }
    if let Some(p) = q.like() {
        qb.push(" AND (a.action ILIKE ")
            .push_bind(p.clone())
            .push(" OR u.name ILIKE ")
            .push_bind(p.clone())
            .push(" OR a.entity_type ILIKE ")
            .push_bind(p)
            .push(")");
    }
    qb.push(" ORDER BY a.id DESC LIMIT ")
        .push_bind(q.page_size)
        .push(" OFFSET ")
        .push_bind(q.offset());
    let rows = qb.build().fetch_all(pool).await?;
    let total: i64 = rows.first().map(|r| r.get("total_count")).unwrap_or(0);
    let items = rows
        .iter()
        .map(<AuditListRow as sqlx::FromRow<_>>::from_row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(crate::paging::PageResult { items, total })
}
