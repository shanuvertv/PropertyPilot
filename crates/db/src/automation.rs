//! Reminder rules, dispatch ledger and notifications (spec §8, §15).

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection, PgExecutor, PgPool};
use uuid::Uuid;

use crate::DbResult;

// ---------------------------------------------------------------- reminder rules

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct RuleRow {
    pub id: Uuid,
    pub days_before: i32,
    pub label: String,
    pub notify_in_app: bool,
    pub email_assigned_employee: bool,
    pub mark_urgent: bool,
    pub active: bool,
    pub sort_order: i32,
}

pub async fn rules<'e>(ex: impl PgExecutor<'e>, active_only: bool) -> DbResult<Vec<RuleRow>> {
    sqlx::query_as(
        "SELECT id, days_before, label, notify_in_app, email_assigned_employee, mark_urgent, active, sort_order
           FROM reminder_rules WHERE ($1 = FALSE OR active) ORDER BY days_before DESC",
    )
    .bind(active_only)
    .fetch_all(ex)
    .await
}

pub struct RuleInput<'a> {
    pub id: Option<Uuid>,
    pub days_before: i32,
    pub label: &'a str,
    pub notify_in_app: bool,
    pub email_assigned_employee: bool,
    pub mark_urgent: bool,
    pub active: bool,
}

/// Replaces the schedule: rows not in `items` are deactivated (dispatch history is kept).
pub async fn save_rules(conn: &mut PgConnection, items: &[RuleInput<'_>]) -> DbResult<()> {
    let keep: Vec<Uuid> = items.iter().filter_map(|i| i.id).collect();
    sqlx::query("UPDATE reminder_rules SET active = FALSE WHERE NOT (id = ANY($1))")
        .bind(&keep)
        .execute(&mut *conn)
        .await?;
    for (i, item) in items.iter().enumerate() {
        let sort = (i as i32 + 1) * 10;
        match item.id {
            Some(id) => {
                sqlx::query(
                    "UPDATE reminder_rules SET days_before = $2, label = $3, notify_in_app = $4, email_assigned_employee = $5,
                            mark_urgent = $6, active = $7, sort_order = $8 WHERE id = $1",
                )
                .bind(id)
                .bind(item.days_before)
                .bind(item.label)
                .bind(item.notify_in_app)
                .bind(item.email_assigned_employee)
                .bind(item.mark_urgent)
                .bind(item.active)
                .bind(sort)
                .execute(&mut *conn)
                .await?;
            }
            None => {
                sqlx::query(
                    "INSERT INTO reminder_rules (days_before, label, notify_in_app, email_assigned_employee, mark_urgent, active, sort_order)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                )
                .bind(item.days_before)
                .bind(item.label)
                .bind(item.notify_in_app)
                .bind(item.email_assigned_employee)
                .bind(item.mark_urgent)
                .bind(item.active)
                .bind(sort)
                .execute(&mut *conn)
                .await?;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- dispatches

pub async fn dispatched<'e>(ex: impl PgExecutor<'e>) -> DbResult<Vec<(Uuid, Uuid)>> {
    sqlx::query_as("SELECT contract_id, rule_id FROM reminder_dispatches")
        .fetch_all(ex)
        .await
}

pub async fn record_dispatch<'e>(
    ex: impl PgExecutor<'e>,
    contract_id: Uuid,
    rule_id: Uuid,
    skipped: bool,
) -> DbResult<()> {
    sqlx::query(
        "INSERT INTO reminder_dispatches (contract_id, rule_id, skipped) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(contract_id)
    .bind(rule_id)
    .bind(skipped)
    .execute(ex)
    .await
    .map(|_| ())
}

/// Snapshot of ACTIVE contracts for the planner.
#[derive(Debug, Clone, FromRow)]
pub struct SweepRow {
    pub id: Uuid,
    pub remaining_days: i32,
    pub has_open_case: bool,
    pub assigned_employee_id: Option<Uuid>,
    pub case_assigned_employee_id: Option<Uuid>,
    pub case_id: Option<Uuid>,
    pub contract_number: String,
    pub tenant_name: String,
    pub building_name: String,
    pub unit_numbers: String,
    pub end_date: chrono::NaiveDate,
}

pub async fn sweep_rows(pool: &PgPool) -> DbResult<Vec<SweepRow>> {
    sqlx::query_as(
        "SELECT c.id, e.remaining_days, e.renewal_in_progress AS has_open_case, c.assigned_employee_id,
                rc.assigned_employee_id AS case_assigned_employee_id, rc.id AS case_id,
                c.contract_number, t.name AS tenant_name, b.name AS building_name, c.end_date,
                (SELECT COALESCE(string_agg(u.unit_number, ', ' ORDER BY u.unit_number), '')
                   FROM contract_units cu JOIN units u ON u.id = cu.unit_id WHERE cu.contract_id = c.id) AS unit_numbers
           FROM contracts c
           JOIN v_contract_expiry e ON e.contract_id = c.id
           JOIN tenants t ON t.id = c.tenant_id
           JOIN buildings b ON b.id = c.building_id
           LEFT JOIN renewal_cases rc ON rc.contract_id = c.id AND rc.status NOT IN ('RENEWAL_COMPLETED', 'CLOSED')
          WHERE c.status = 'ACTIVE'",
    )
    .fetch_all(pool)
    .await
}

// ---------------------------------------------------------------- notifications

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct NotificationRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: String,
    pub title: String,
    pub body: Option<String>,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

pub struct NewNotification<'a> {
    pub user_id: Uuid,
    pub kind: &'a str,
    pub title: &'a str,
    pub body: Option<&'a str>,
    pub entity_type: &'a str,
    pub entity_id: Uuid,
    /// When set, the same key never notifies the same user twice.
    pub dedupe_key: Option<&'a str>,
}

/// Returns true when a row was inserted (false = deduplicated).
pub async fn notify<'e>(ex: impl PgExecutor<'e>, n: &NewNotification<'_>) -> DbResult<bool> {
    let r = sqlx::query(
        "INSERT INTO notifications (user_id, kind, title, body, entity_type, entity_id, dedupe_key)
         VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING",
    )
    .bind(n.user_id)
    .bind(n.kind)
    .bind(n.title)
    .bind(n.body)
    .bind(n.entity_type)
    .bind(n.entity_id)
    .bind(n.dedupe_key)
    .execute(ex)
    .await?;
    Ok(r.rows_affected() > 0)
}

pub async fn list<'e>(
    ex: impl PgExecutor<'e>,
    user_id: Uuid,
    unread_only: bool,
    limit: i64,
) -> DbResult<Vec<NotificationRow>> {
    sqlx::query_as(
        "SELECT id, user_id, kind, title, body, entity_type, entity_id, created_at, read_at FROM notifications
          WHERE user_id = $1 AND ($2 = FALSE OR read_at IS NULL)
          ORDER BY (read_at IS NULL) DESC, created_at DESC LIMIT $3",
    )
    .bind(user_id)
    .bind(unread_only)
    .bind(limit.clamp(1, 500))
    .fetch_all(ex)
    .await
}

pub async fn unread_count<'e>(ex: impl PgExecutor<'e>, user_id: Uuid) -> DbResult<i64> {
    sqlx::query_scalar("SELECT count(*) FROM notifications WHERE user_id = $1 AND read_at IS NULL")
        .bind(user_id)
        .fetch_one(ex)
        .await
}

pub async fn mark_read<'e>(ex: impl PgExecutor<'e>, user_id: Uuid, id: Uuid) -> DbResult<()> {
    sqlx::query("UPDATE notifications SET read_at = now() WHERE id = $1 AND user_id = $2 AND read_at IS NULL")
        .bind(id)
        .bind(user_id)
        .execute(ex)
        .await
        .map(|_| ())
}

pub async fn mark_all_read<'e>(ex: impl PgExecutor<'e>, user_id: Uuid) -> DbResult<u64> {
    sqlx::query("UPDATE notifications SET read_at = now() WHERE user_id = $1 AND read_at IS NULL")
        .bind(user_id)
        .execute(ex)
        .await
        .map(|r| r.rows_affected())
}

/// Ids of active Admins (fallback audience when a contract has nobody assigned).
pub async fn admin_ids<'e>(ex: impl PgExecutor<'e>) -> DbResult<Vec<Uuid>> {
    sqlx::query_scalar("SELECT id FROM users WHERE role = 'ADMIN' AND active")
        .fetch_all(ex)
        .await
}

#[derive(Debug, Clone, FromRow)]
pub struct DueFollowUp {
    pub id: Uuid,
    pub case_id: Uuid,
    pub assigned_employee_id: Option<Uuid>,
    pub due_date: chrono::NaiveDate,
    pub days_until_due: i32,
    pub follow_up_type: String,
    pub tenant_name: String,
    pub contract_number: String,
}

/// Open follow-ups due today or overdue, for the daily notifications.
pub async fn due_follow_ups(pool: &PgPool) -> DbResult<Vec<DueFollowUp>> {
    sqlx::query_as(
        "SELECT f.id, f.case_id, f.assigned_employee_id, f.due_date, (f.due_date - CURRENT_DATE)::int AS days_until_due,
                f.follow_up_type, t.name AS tenant_name, c.contract_number
           FROM follow_ups f JOIN renewal_cases rc ON rc.id = f.case_id JOIN contracts c ON c.id = rc.contract_id
           JOIN tenants t ON t.id = c.tenant_id
          WHERE f.status = 'OPEN' AND f.due_date <= CURRENT_DATE",
    )
    .fetch_all(pool)
    .await
}

#[derive(Debug, Clone, FromRow)]
pub struct PendingCase {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub assigned_employee_id: Option<Uuid>,
    pub status: String,
    pub notice_status: String,
    pub remaining_days: i32,
    pub tenant_name: String,
    pub contract_number: String,
    pub days_since_notice: Option<i32>,
}

/// Open cases whose notice is still pending, or whose tenant has not answered.
pub async fn pending_cases(pool: &PgPool) -> DbResult<Vec<PendingCase>> {
    sqlx::query_as(
        "SELECT rc.id, rc.contract_id, rc.assigned_employee_id, rc.status, rc.notice_status, e.remaining_days,
                t.name AS tenant_name, c.contract_number,
                (SELECT (CURRENT_DATE - max(n.sent_at)::date)::int FROM renewal_notices n WHERE n.case_id = rc.id AND n.status <> 'DRAFT') AS days_since_notice
           FROM renewal_cases rc JOIN contracts c ON c.id = rc.contract_id JOIN tenants t ON t.id = c.tenant_id
           JOIN v_contract_expiry e ON e.contract_id = c.id
          WHERE rc.status NOT IN ('RENEWAL_COMPLETED', 'CLOSED') AND c.status = 'ACTIVE'",
    )
    .fetch_all(pool)
    .await
}
