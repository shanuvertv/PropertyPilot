//! Email templates, outbound messages and renewal notices (spec §7, §9, §10).

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection, PgExecutor, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::paging::{ListQuery, PageResult};
use crate::DbResult;

// ---------------------------------------------------------------- templates

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct TemplateRow {
    pub id: Uuid,
    pub key: String,
    pub name: String,
    pub subject: String,
    pub body_text: String,
    pub active: bool,
    pub updated_at: DateTime<Utc>,
}

pub async fn templates<'e>(ex: impl PgExecutor<'e>) -> DbResult<Vec<TemplateRow>> {
    sqlx::query_as("SELECT id, key, name, subject, body_text, active, updated_at FROM email_templates ORDER BY name")
        .fetch_all(ex)
        .await
}

pub async fn template<'e>(ex: impl PgExecutor<'e>, key: &str) -> DbResult<Option<TemplateRow>> {
    sqlx::query_as("SELECT id, key, name, subject, body_text, active, updated_at FROM email_templates WHERE key = $1")
        .bind(key)
        .fetch_optional(ex)
        .await
}

pub async fn update_template<'e>(
    ex: impl PgExecutor<'e>,
    key: &str,
    name: &str,
    subject: &str,
    body_text: &str,
    active: bool,
    by: Uuid,
) -> DbResult<()> {
    sqlx::query(
        "UPDATE email_templates SET name = $2, subject = $3, body_text = $4, active = $5, updated_by = $6, updated_at = now() WHERE key = $1",
    )
    .bind(key)
    .bind(name)
    .bind(subject)
    .bind(body_text)
    .bind(active)
    .bind(by)
    .execute(ex)
    .await
    .map(|_| ())
}

// ---------------------------------------------------------------- messages

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct MessageRow {
    pub id: Uuid,
    pub email_type: String,
    pub tenant_id: Option<Uuid>,
    pub tenant_name: Option<String>,
    pub contract_id: Option<Uuid>,
    pub contract_number: Option<String>,
    pub case_id: Option<Uuid>,
    pub to_addresses: Vec<String>,
    pub cc_addresses: Vec<String>,
    pub subject: String,
    pub body_text: String,
    pub body_html: String,
    pub status: String,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub provider: Option<String>,
    pub provider_message_id: Option<String>,
    pub sent_by: Option<Uuid>,
    pub sent_by_name: Option<String>,
    pub queued_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub attachment_count: i64,
}

const SELECT: &str = "SELECT m.id, m.email_type, m.tenant_id, t.name AS tenant_name, m.contract_id, c.contract_number, m.case_id,
       m.to_addresses, m.cc_addresses, m.subject, m.body_text, m.body_html, m.status, m.attempts, m.last_error,
       m.provider, m.provider_message_id, m.sent_by, u.name AS sent_by_name, m.queued_at, m.sent_at,
       (SELECT count(*) FROM email_attachments a WHERE a.message_id = m.id) AS attachment_count
  FROM email_messages m
  LEFT JOIN tenants t ON t.id = m.tenant_id
  LEFT JOIN contracts c ON c.id = m.contract_id
  LEFT JOIN users u ON u.id = m.sent_by";

const SORTS: &[(&str, &str)] = &[
    ("queued_at", "m.queued_at"),
    ("status", "m.status"),
    ("tenant_name", "t.name"),
];

#[derive(Debug, Clone, Default)]
pub struct MessageFilter {
    pub tenant_id: Option<Uuid>,
    pub contract_id: Option<Uuid>,
    pub case_id: Option<Uuid>,
    pub status: Option<String>,
    pub email_type: Option<String>,
}

pub async fn list_messages(
    pool: &PgPool,
    f: &MessageFilter,
    q: &ListQuery,
) -> DbResult<PageResult<MessageRow>> {
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(SELECT.replacen(
        "SELECT ",
        "SELECT count(*) OVER () AS total_count, ",
        1,
    ));
    qb.push(" WHERE TRUE");
    if let Some(t) = f.tenant_id {
        qb.push(" AND m.tenant_id = ").push_bind(t);
    }
    if let Some(c) = f.contract_id {
        qb.push(" AND m.contract_id = ").push_bind(c);
    }
    if let Some(c) = f.case_id {
        qb.push(" AND m.case_id = ").push_bind(c);
    }
    if let Some(s) = &f.status {
        qb.push(" AND m.status = ").push_bind(s.clone());
    }
    if let Some(t) = &f.email_type {
        qb.push(" AND m.email_type = ").push_bind(t.clone());
    }
    if let Some(p) = q.like() {
        qb.push(" AND (m.subject ILIKE ")
            .push_bind(p.clone())
            .push(" OR t.name ILIKE ")
            .push_bind(p.clone())
            .push(" OR array_to_string(m.to_addresses, ',') ILIKE ")
            .push_bind(p)
            .push(")");
    }
    let mut q2 = q.clone();
    if q2.sort.is_none() {
        q2.sort = Some("queued_at".into());
        q2.dir = crate::paging::SortDir::Desc;
    }
    qb.push(q2.order_by(SORTS))
        .push(" LIMIT ")
        .push_bind(q2.page_size)
        .push(" OFFSET ")
        .push_bind(q2.offset());
    let rows = qb.build().fetch_all(pool).await?;
    let total: i64 = rows.first().map(|r| r.get("total_count")).unwrap_or(0);
    let items = rows
        .iter()
        .map(MessageRow::from_row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PageResult { items, total })
}

pub async fn find_message<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<Option<MessageRow>> {
    sqlx::query_as(&format!("{SELECT} WHERE m.id = $1"))
        .bind(id)
        .fetch_optional(ex)
        .await
}

pub struct NewMessage<'a> {
    pub email_type: &'a str,
    pub tenant_id: Option<Uuid>,
    pub contract_id: Option<Uuid>,
    pub case_id: Option<Uuid>,
    pub to: &'a [String],
    pub cc: &'a [String],
    pub subject: &'a str,
    pub body_text: &'a str,
    pub body_html: &'a str,
    pub sent_by: Uuid,
}

pub async fn insert_message<'e>(ex: impl PgExecutor<'e>, m: &NewMessage<'_>) -> DbResult<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO email_messages (email_type, tenant_id, contract_id, case_id, to_addresses, cc_addresses, subject, body_text, body_html, sent_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id",
    )
    .bind(m.email_type)
    .bind(m.tenant_id)
    .bind(m.contract_id)
    .bind(m.case_id)
    .bind(m.to)
    .bind(m.cc)
    .bind(m.subject)
    .bind(m.body_text)
    .bind(m.body_html)
    .bind(m.sent_by)
    .fetch_one(ex)
    .await
}

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct AttachmentRow {
    pub id: Uuid,
    pub message_id: Uuid,
    pub document_id: Option<Uuid>,
    pub file_name: String,
    pub content_type: String,
    pub storage_key: String,
}

pub async fn insert_attachment<'e>(
    ex: impl PgExecutor<'e>,
    message_id: Uuid,
    document_id: Option<Uuid>,
    file_name: &str,
    content_type: &str,
    storage_key: &str,
) -> DbResult<()> {
    sqlx::query(
        "INSERT INTO email_attachments (message_id, document_id, file_name, content_type, storage_key) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(message_id)
    .bind(document_id)
    .bind(file_name)
    .bind(content_type)
    .bind(storage_key)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn attachments<'e>(
    ex: impl PgExecutor<'e>,
    message_id: Uuid,
) -> DbResult<Vec<AttachmentRow>> {
    sqlx::query_as("SELECT id, message_id, document_id, file_name, content_type, storage_key FROM email_attachments WHERE message_id = $1")
        .bind(message_id)
        .fetch_all(ex)
        .await
}

/// Claims due QUEUED messages for one sender pass (skip-locked so parallel senders never double-send).
pub async fn claim_due(conn: &mut PgConnection, limit: i64) -> DbResult<Vec<MessageRow>> {
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM email_messages WHERE status = 'QUEUED' AND next_attempt_at <= now()
          ORDER BY queued_at LIMIT $1 FOR UPDATE SKIP LOCKED",
    )
    .bind(limit)
    .fetch_all(&mut *conn)
    .await?;
    if ids.is_empty() {
        return Ok(vec![]);
    }
    // Push the next attempt out so a crashed sender does not spin on the same rows.
    sqlx::query("UPDATE email_messages SET next_attempt_at = now() + interval '5 minutes', attempts = attempts + 1 WHERE id = ANY($1)")
        .bind(&ids)
        .execute(&mut *conn)
        .await?;
    sqlx::query_as(&format!(
        "{SELECT} WHERE m.id = ANY($1) ORDER BY m.queued_at"
    ))
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await
}

pub async fn mark_sent<'e>(
    ex: impl PgExecutor<'e>,
    id: Uuid,
    provider: &str,
    provider_message_id: Option<&str>,
) -> DbResult<()> {
    sqlx::query(
        "UPDATE email_messages SET status = 'SENT', sent_at = now(), provider = $2, provider_message_id = $3, last_error = NULL WHERE id = $1",
    )
    .bind(id)
    .bind(provider)
    .bind(provider_message_id)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn mark_failed<'e>(
    ex: impl PgExecutor<'e>,
    id: Uuid,
    error: &str,
    give_up: bool,
) -> DbResult<()> {
    sqlx::query(
        "UPDATE email_messages SET status = CASE WHEN $3 THEN 'FAILED' ELSE 'QUEUED' END, last_error = $2,
                next_attempt_at = now() + interval '10 minutes' WHERE id = $1",
    )
    .bind(id)
    .bind(error)
    .bind(give_up)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn requeue<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<()> {
    sqlx::query("UPDATE email_messages SET status = 'QUEUED', attempts = 0, next_attempt_at = now(), last_error = NULL WHERE id = $1 AND status = 'FAILED'")
        .bind(id)
        .execute(ex)
        .await
        .map(|_| ())
}

// ---------------------------------------------------------------- notices

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct NoticeRow {
    pub id: Uuid,
    pub case_id: Uuid,
    pub status: String,
    pub proposed_period: Option<String>,
    pub other_terms: Option<String>,
    pub subject: String,
    pub body_text: String,
    pub recipient: Option<String>,
    pub cc_addresses: Vec<String>,
    pub pdf_document_id: Option<Uuid>,
    pub email_message_id: Option<Uuid>,
    pub email_status: Option<String>,
    pub sent_by: Option<Uuid>,
    pub sent_by_name: Option<String>,
    pub sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const NOTICE_SELECT: &str = "SELECT n.id, n.case_id, n.status, n.proposed_period, n.other_terms, n.subject, n.body_text, n.recipient, n.cc_addresses,
       n.pdf_document_id, n.email_message_id, m.status AS email_status, n.sent_by, u.name AS sent_by_name, n.sent_at, n.created_at, n.updated_at
  FROM renewal_notices n LEFT JOIN email_messages m ON m.id = n.email_message_id LEFT JOIN users u ON u.id = n.sent_by";

pub async fn notices_for_case<'e>(
    ex: impl PgExecutor<'e>,
    case_id: Uuid,
) -> DbResult<Vec<NoticeRow>> {
    sqlx::query_as(&format!(
        "{NOTICE_SELECT} WHERE n.case_id = $1 ORDER BY n.created_at DESC"
    ))
    .bind(case_id)
    .fetch_all(ex)
    .await
}

pub async fn find_notice<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<Option<NoticeRow>> {
    sqlx::query_as(&format!("{NOTICE_SELECT} WHERE n.id = $1"))
        .bind(id)
        .fetch_optional(ex)
        .await
}

pub async fn draft_for_case<'e>(
    ex: impl PgExecutor<'e>,
    case_id: Uuid,
) -> DbResult<Option<NoticeRow>> {
    sqlx::query_as(&format!(
        "{NOTICE_SELECT} WHERE n.case_id = $1 AND n.status = 'DRAFT'"
    ))
    .bind(case_id)
    .fetch_optional(ex)
    .await
}

pub struct NoticeDraft<'a> {
    pub proposed_period: Option<&'a str>,
    pub other_terms: Option<&'a str>,
    pub subject: &'a str,
    pub body_text: &'a str,
    pub recipient: Option<&'a str>,
    pub cc: &'a [String],
}

pub async fn upsert_draft<'e>(
    ex: impl PgExecutor<'e>,
    case_id: Uuid,
    d: &NoticeDraft<'_>,
    by: Uuid,
) -> DbResult<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO renewal_notices (case_id, proposed_period, other_terms, subject, body_text, recipient, cc_addresses, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (case_id) WHERE status = 'DRAFT' DO UPDATE SET proposed_period = EXCLUDED.proposed_period,
             other_terms = EXCLUDED.other_terms, subject = EXCLUDED.subject, body_text = EXCLUDED.body_text,
             recipient = EXCLUDED.recipient, cc_addresses = EXCLUDED.cc_addresses, pdf_document_id = NULL, updated_at = now()
         RETURNING id",
    )
    .bind(case_id)
    .bind(d.proposed_period)
    .bind(d.other_terms)
    .bind(d.subject)
    .bind(d.body_text)
    .bind(d.recipient)
    .bind(d.cc)
    .bind(by)
    .fetch_one(ex)
    .await
}

pub async fn set_notice_pdf<'e>(
    ex: impl PgExecutor<'e>,
    id: Uuid,
    document_id: Uuid,
) -> DbResult<()> {
    sqlx::query("UPDATE renewal_notices SET pdf_document_id = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(document_id)
        .execute(ex)
        .await
        .map(|_| ())
}

pub async fn mark_notice_sent<'e>(
    ex: impl PgExecutor<'e>,
    id: Uuid,
    email_message_id: Uuid,
    recipient: &str,
    cc: &[String],
    by: Uuid,
) -> DbResult<()> {
    sqlx::query(
        "UPDATE renewal_notices SET status = 'SENT', email_message_id = $2, recipient = $3, cc_addresses = $4, sent_by = $5, sent_at = now(), updated_at = now()
          WHERE id = $1",
    )
    .bind(id)
    .bind(email_message_id)
    .bind(recipient)
    .bind(cc)
    .bind(by)
    .execute(ex)
    .await
    .map(|_| ())
}

/// Called by the sender: mirrors the email outcome onto the notice and its case.
pub async fn sync_notice_delivery(
    conn: &mut PgConnection,
    email_message_id: Uuid,
    delivered: bool,
) -> DbResult<()> {
    let status = if delivered { "DELIVERED" } else { "FAILED" };
    let case_ids: Vec<Uuid> = sqlx::query_scalar(
        "UPDATE renewal_notices SET status = $2, updated_at = now() WHERE email_message_id = $1 RETURNING case_id",
    )
    .bind(email_message_id)
    .bind(status)
    .fetch_all(&mut *conn)
    .await?;
    for case_id in case_ids {
        sqlx::query(
            "UPDATE renewal_cases SET notice_status = $2, updated_at = now() WHERE id = $1",
        )
        .bind(case_id)
        .bind(status)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}
