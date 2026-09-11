//! Renewal cases, tenant responses and the per-case checklist (spec §6, §7, §11, §13).

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgConnection, PgExecutor, PgPool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::paging::{ListQuery, PageResult};
use crate::DbResult;

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct CaseRow {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub status: String,
    pub notice_status: String,
    pub assigned_employee_id: Option<Uuid>,
    pub assigned_employee_name: Option<String>,
    pub is_urgent: bool,
    pub latest_response: Option<String>,
    pub latest_response_at: Option<DateTime<Utc>>,
    pub next_follow_up_date: Option<NaiveDate>,
    pub outcome_contract_id: Option<Uuid>,
    pub outcome_contract_number: Option<String>,
    pub notes: Option<String>,
    pub opened_by: Option<Uuid>,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub contract_number: String,
    pub contract_status: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub tenant_id: Uuid,
    pub tenant_name: String,
    pub tenant_contact: Option<String>,
    pub tenant_email: Option<String>,
    pub tenant_mobile: Option<String>,
    pub building_id: Uuid,
    pub building_name: String,
    pub unit_numbers: String,
    pub remaining_days: i32,
    pub band: String,
    pub checklist_total: i64,
    pub checklist_done: i64,
    pub open_follow_ups: i64,
    pub notice_sent_at: Option<DateTime<Utc>>,
    pub notice_recipient: Option<String>,
}

const SELECT: &str = "SELECT rc.id, rc.contract_id, rc.status, rc.notice_status, rc.assigned_employee_id, emp.name AS assigned_employee_name,
       rc.is_urgent, rc.latest_response, rc.latest_response_at, rc.next_follow_up_date, rc.outcome_contract_id,
       oc.contract_number AS outcome_contract_number, rc.notes, rc.opened_by, rc.opened_at, rc.closed_at, rc.updated_at,
       c.contract_number, c.status AS contract_status, c.start_date, c.end_date,
       c.tenant_id, t.name AS tenant_name, t.contact_person AS tenant_contact, t.email AS tenant_email, t.mobile AS tenant_mobile,
       c.building_id, b.name AS building_name,
       (SELECT COALESCE(string_agg(u.unit_number, ', ' ORDER BY u.unit_number), '')
          FROM contract_units cu JOIN units u ON u.id = cu.unit_id WHERE cu.contract_id = c.id) AS unit_numbers,
       e.remaining_days, e.band,
       (SELECT count(*) FROM renewal_checklist_items i WHERE i.case_id = rc.id) AS checklist_total,
       (SELECT count(*) FROM renewal_checklist_items i WHERE i.case_id = rc.id AND i.done) AS checklist_done,
       (SELECT count(*) FROM follow_ups f WHERE f.case_id = rc.id AND f.status = 'OPEN') AS open_follow_ups,
       (SELECT max(n.sent_at) FROM renewal_notices n WHERE n.case_id = rc.id AND n.status <> 'DRAFT') AS notice_sent_at,
       (SELECT n.recipient FROM renewal_notices n WHERE n.case_id = rc.id AND n.status <> 'DRAFT' ORDER BY n.sent_at DESC LIMIT 1) AS notice_recipient
  FROM renewal_cases rc
  JOIN contracts c ON c.id = rc.contract_id
  JOIN tenants t ON t.id = c.tenant_id
  JOIN buildings b ON b.id = c.building_id
  JOIN v_contract_expiry e ON e.contract_id = c.id
  LEFT JOIN users emp ON emp.id = rc.assigned_employee_id
  LEFT JOIN contracts oc ON oc.id = rc.outcome_contract_id";

const SORTS: &[(&str, &str)] = &[
    ("remaining_days", "e.remaining_days"),
    ("opened_at", "rc.opened_at"),
    ("closed_at", "rc.closed_at"),
    ("status", "rc.status"),
    ("tenant_name", "t.name"),
    ("end_date", "c.end_date"),
];

#[derive(Debug, Clone, Default)]
pub struct CaseFilter {
    pub status: Option<Vec<String>>,
    pub notice_status: Option<Vec<String>>,
    pub band: Option<String>,
    pub open_only: bool,
    pub assigned_employee_id: Option<Uuid>,
    pub building_id: Option<Uuid>,
    pub contract_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub latest_response: Option<Vec<String>>,
    pub completed_within_days: Option<i32>,
}

pub async fn list(pool: &PgPool, f: &CaseFilter, q: &ListQuery) -> DbResult<PageResult<CaseRow>> {
    let like = q.like();
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(SELECT.replacen(
        "SELECT ",
        "SELECT count(*) OVER () AS total_count, ",
        1,
    ));
    qb.push(" WHERE TRUE");
    if let Some(s) = &f.status {
        qb.push(" AND rc.status = ANY(")
            .push_bind(s.clone())
            .push(")");
    }
    if f.open_only {
        qb.push(" AND rc.status NOT IN ('RENEWAL_COMPLETED', 'CLOSED')");
    }
    if let Some(ns) = &f.notice_status {
        qb.push(" AND rc.notice_status = ANY(")
            .push_bind(ns.clone())
            .push(")");
    }
    if let Some(b) = &f.band {
        qb.push(" AND e.band = ").push_bind(b.clone());
    }
    if let Some(a) = f.assigned_employee_id {
        qb.push(" AND rc.assigned_employee_id = ").push_bind(a);
    }
    if let Some(b) = f.building_id {
        qb.push(" AND c.building_id = ").push_bind(b);
    }
    if let Some(cid) = f.contract_id {
        qb.push(" AND rc.contract_id = ").push_bind(cid);
    }
    if let Some(t) = f.tenant_id {
        qb.push(" AND c.tenant_id = ").push_bind(t);
    }
    if let Some(r) = &f.latest_response {
        qb.push(" AND rc.latest_response = ANY(")
            .push_bind(r.clone())
            .push(")");
    }
    if let Some(d) = f.completed_within_days {
        qb.push(" AND rc.outcome_contract_id IS NOT NULL AND rc.closed_at >= now() - make_interval(days => ")
            .push_bind(d)
            .push(")");
    }
    if let Some(p) = &like {
        qb.push(" AND (c.contract_number ILIKE ")
            .push_bind(p.clone())
            .push(" OR t.name ILIKE ")
            .push_bind(p.clone())
            .push(" OR b.name ILIKE ")
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
        .map(CaseRow::from_row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PageResult { items, total })
}

pub async fn find<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<Option<CaseRow>> {
    sqlx::query_as(&format!("{SELECT} WHERE rc.id = $1"))
        .bind(id)
        .fetch_optional(ex)
        .await
}

pub async fn open_for_contract<'e>(
    ex: impl PgExecutor<'e>,
    contract_id: Uuid,
) -> DbResult<Option<CaseRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE rc.contract_id = $1 AND rc.status NOT IN ('RENEWAL_COMPLETED', 'CLOSED')"
    ))
    .bind(contract_id)
    .fetch_optional(ex)
    .await
}

pub async fn for_tenant<'e>(ex: impl PgExecutor<'e>, tenant_id: Uuid) -> DbResult<Vec<CaseRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE c.tenant_id = $1 ORDER BY rc.opened_at DESC"
    ))
    .bind(tenant_id)
    .fetch_all(ex)
    .await
}

/// Opens a case and copies the active checklist template into it.
pub async fn insert(
    conn: &mut PgConnection,
    contract_id: Uuid,
    assigned_employee_id: Option<Uuid>,
    is_urgent: bool,
    opened_by: Uuid,
) -> DbResult<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO renewal_cases (contract_id, assigned_employee_id, is_urgent, opened_by) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(contract_id)
    .bind(assigned_employee_id)
    .bind(is_urgent)
    .bind(opened_by)
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO renewal_checklist_items (case_id, key, label, sort_order, required)
         SELECT $1, key, label, sort_order, required FROM checklist_template_items WHERE active ORDER BY sort_order",
    )
    .bind(id)
    .execute(&mut *conn)
    .await?;
    Ok(id)
}

pub async fn set_status<'e>(ex: impl PgExecutor<'e>, id: Uuid, status: &str) -> DbResult<()> {
    sqlx::query(
        "UPDATE renewal_cases SET status = $2, updated_at = now(),
                closed_at = CASE WHEN $2 IN ('RENEWAL_COMPLETED', 'CLOSED') THEN COALESCE(closed_at, now()) ELSE closed_at END
          WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn set_notice_status<'e>(
    ex: impl PgExecutor<'e>,
    id: Uuid,
    notice_status: &str,
) -> DbResult<()> {
    sqlx::query("UPDATE renewal_cases SET notice_status = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(notice_status)
        .execute(ex)
        .await
        .map(|_| ())
}

pub async fn set_assigned<'e>(
    ex: impl PgExecutor<'e>,
    id: Uuid,
    employee: Option<Uuid>,
) -> DbResult<()> {
    sqlx::query(
        "UPDATE renewal_cases SET assigned_employee_id = $2, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(employee)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn set_urgent<'e>(ex: impl PgExecutor<'e>, id: Uuid, urgent: bool) -> DbResult<()> {
    sqlx::query("UPDATE renewal_cases SET is_urgent = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(urgent)
        .execute(ex)
        .await
        .map(|_| ())
}

pub async fn set_notes<'e>(ex: impl PgExecutor<'e>, id: Uuid, notes: Option<&str>) -> DbResult<()> {
    sqlx::query("UPDATE renewal_cases SET notes = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(notes)
        .execute(ex)
        .await
        .map(|_| ())
}

pub async fn set_outcome<'e>(
    ex: impl PgExecutor<'e>,
    id: Uuid,
    new_contract_id: Uuid,
) -> DbResult<()> {
    sqlx::query(
        "UPDATE renewal_cases SET outcome_contract_id = $2, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(new_contract_id)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn set_next_follow_up<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<()> {
    sqlx::query(
        "UPDATE renewal_cases SET next_follow_up_date = (SELECT min(due_date) FROM follow_ups WHERE case_id = $1 AND status = 'OPEN'),
                updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .execute(ex)
    .await
    .map(|_| ())
}

// ---------------------------------------------------------------- responses

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct ResponseRow {
    pub id: Uuid,
    pub case_id: Uuid,
    pub response: String,
    pub response_date: NaiveDate,
    pub notes: Option<String>,
    pub follow_up_date: Option<NaiveDate>,
    pub recorded_by: Option<Uuid>,
    pub recorded_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub async fn responses<'e>(ex: impl PgExecutor<'e>, case_id: Uuid) -> DbResult<Vec<ResponseRow>> {
    sqlx::query_as(
        "SELECT r.id, r.case_id, r.response, r.response_date, r.notes, r.follow_up_date, r.recorded_by, u.name AS recorded_by_name, r.created_at
           FROM renewal_responses r LEFT JOIN users u ON u.id = r.recorded_by
          WHERE r.case_id = $1 ORDER BY r.created_at DESC",
    )
    .bind(case_id)
    .fetch_all(ex)
    .await
}

pub async fn insert_response(
    conn: &mut PgConnection,
    case_id: Uuid,
    response: &str,
    response_date: NaiveDate,
    notes: Option<&str>,
    follow_up_date: Option<NaiveDate>,
    recorded_by: Uuid,
) -> DbResult<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO renewal_responses (case_id, response, response_date, notes, follow_up_date, recorded_by)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(case_id)
    .bind(response)
    .bind(response_date)
    .bind(notes)
    .bind(follow_up_date)
    .bind(recorded_by)
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query("UPDATE renewal_cases SET latest_response = $2, latest_response_at = now(), updated_at = now() WHERE id = $1")
        .bind(case_id)
        .bind(response)
        .execute(&mut *conn)
        .await?;
    Ok(id)
}

// ---------------------------------------------------------------- checklist

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct ChecklistItemRow {
    pub id: Uuid,
    pub case_id: Uuid,
    pub key: Option<String>,
    pub label: String,
    pub sort_order: i32,
    pub required: bool,
    pub done: bool,
    pub done_by: Option<Uuid>,
    pub done_by_name: Option<String>,
    pub done_at: Option<DateTime<Utc>>,
}

pub async fn checklist<'e>(
    ex: impl PgExecutor<'e>,
    case_id: Uuid,
) -> DbResult<Vec<ChecklistItemRow>> {
    sqlx::query_as(
        "SELECT i.id, i.case_id, i.key, i.label, i.sort_order, i.required, i.done, i.done_by, u.name AS done_by_name, i.done_at
           FROM renewal_checklist_items i LEFT JOIN users u ON u.id = i.done_by
          WHERE i.case_id = $1 ORDER BY i.sort_order",
    )
    .bind(case_id)
    .fetch_all(ex)
    .await
}

pub async fn set_checklist_item<'e>(
    ex: impl PgExecutor<'e>,
    item_id: Uuid,
    done: bool,
    by: Uuid,
) -> DbResult<Option<ChecklistItemRow>> {
    sqlx::query_as(
        "UPDATE renewal_checklist_items i SET done = $2, done_by = CASE WHEN $2 THEN $3 END, done_at = CASE WHEN $2 THEN now() END
          WHERE i.id = $1
          RETURNING i.id, i.case_id, i.key, i.label, i.sort_order, i.required, i.done, i.done_by,
                    (SELECT name FROM users WHERE id = i.done_by) AS done_by_name, i.done_at",
    )
    .bind(item_id)
    .bind(done)
    .bind(by)
    .fetch_optional(ex)
    .await
}

/// Ticks the item with the given system key on a case (no-op if the Admin removed it).
pub async fn tick_by_key<'e>(
    ex: impl PgExecutor<'e>,
    case_id: Uuid,
    key: &str,
    by: Uuid,
) -> DbResult<()> {
    sqlx::query(
        "UPDATE renewal_checklist_items SET done = TRUE, done_by = $3, done_at = now()
          WHERE case_id = $1 AND key = $2 AND NOT done",
    )
    .bind(case_id)
    .bind(key)
    .bind(by)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn required_open_items<'e>(
    ex: impl PgExecutor<'e>,
    case_id: Uuid,
) -> DbResult<Vec<String>> {
    sqlx::query_scalar(
        "SELECT label FROM renewal_checklist_items WHERE case_id = $1 AND required AND NOT done ORDER BY sort_order",
    )
    .bind(case_id)
    .fetch_all(ex)
    .await
}

// ---------------------------------------------------------------- template (Admin)

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct TemplateItemRow {
    pub id: Uuid,
    pub key: Option<String>,
    pub label: String,
    pub sort_order: i32,
    pub required: bool,
    pub active: bool,
}

pub async fn template<'e>(ex: impl PgExecutor<'e>) -> DbResult<Vec<TemplateItemRow>> {
    sqlx::query_as("SELECT id, key, label, sort_order, required, active FROM checklist_template_items ORDER BY sort_order")
        .fetch_all(ex)
        .await
}

pub struct TemplateItemInput<'a> {
    pub id: Option<Uuid>,
    pub label: &'a str,
    pub sort_order: i32,
    pub required: bool,
    pub active: bool,
}

/// Replaces the template: existing ids are updated, new items inserted, missing ones deactivated.
pub async fn save_template(
    conn: &mut PgConnection,
    items: &[TemplateItemInput<'_>],
) -> DbResult<()> {
    let keep: Vec<Uuid> = items.iter().filter_map(|i| i.id).collect();
    sqlx::query("UPDATE checklist_template_items SET active = FALSE WHERE NOT (id = ANY($1))")
        .bind(&keep)
        .execute(&mut *conn)
        .await?;
    for item in items {
        match item.id {
            Some(id) => {
                sqlx::query("UPDATE checklist_template_items SET label = $2, sort_order = $3, required = $4, active = $5 WHERE id = $1")
                    .bind(id)
                    .bind(item.label)
                    .bind(item.sort_order)
                    .bind(item.required)
                    .bind(item.active)
                    .execute(&mut *conn)
                    .await?;
            }
            None => {
                sqlx::query("INSERT INTO checklist_template_items (label, sort_order, required, active) VALUES ($1, $2, $3, $4)")
                    .bind(item.label)
                    .bind(item.sort_order)
                    .bind(item.required)
                    .bind(item.active)
                    .execute(&mut *conn)
                    .await?;
            }
        }
    }
    Ok(())
}
