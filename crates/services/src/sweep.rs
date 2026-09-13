//! Daily expiry sweep: applies the pure planner from `renewal_core::sweep` (spec §8).

use chrono::NaiveDate;
use renewal_core::{plan_sweep, ReminderRule, SweepAction, SweepContract, SweepInput, Thresholds};
use renewal_db::automation::{self, SweepRow};
use renewal_db::{contracts, renewals, settings, users, PgPool};
use serde::Serialize;
use uuid::Uuid;

use crate::error::ServiceResult;
use crate::notifications::{audience, notify_many};
use crate::templates;
use crate::{contracts as contract_svc, emails};

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SweepSummary {
    pub contracts_checked: usize,
    pub reminders_fired: usize,
    pub reminders_skipped: usize,
    pub cases_opened: usize,
    pub contracts_expired: usize,
    pub notifications_created: usize,
    pub emails_queued: usize,
    pub follow_up_alerts: usize,
    pub notice_alerts: usize,
    pub response_alerts: usize,
    pub cheque_alerts: usize,
}

fn fmt_date(d: NaiveDate) -> String {
    d.format("%d %b %Y").to_string()
}

/// Runs the whole daily sweep. Safe to run several times a day: every step is idempotent
/// (dispatch ledger, one open case per contract, deduplicated notifications).
pub async fn run(pool: &PgPool, actor: Option<&crate::Session>) -> ServiceResult<SweepSummary> {
    let mut summary = SweepSummary::default();
    let today = renewal_db::today(pool).await?;
    let thresholds: Thresholds = settings::get(pool, settings::EXPIRY_THRESHOLDS)
        .await?
        .unwrap_or_default();
    let auto_open: bool = settings::get(pool, "renewals.autoOpenCase")
        .await?
        .unwrap_or(true);

    let rows = automation::sweep_rows(pool).await?;
    let rule_rows = automation::rules(pool, true).await?;
    let rules: Vec<ReminderRule> = rule_rows
        .iter()
        .map(|r| ReminderRule {
            id: r.id,
            days_before: r.days_before as i64,
            label: r.label.clone(),
            notify_in_app: r.notify_in_app,
            email_assigned_employee: r.email_assigned_employee,
            mark_urgent: r.mark_urgent,
        })
        .collect();
    let dispatched = automation::dispatched(pool).await?;
    let contracts_snapshot: Vec<SweepContract> = rows
        .iter()
        .map(|r| SweepContract {
            id: r.id,
            remaining_days: r.remaining_days as i64,
            has_open_case: r.has_open_case,
        })
        .collect();
    summary.contracts_checked = rows.len();

    let actions = plan_sweep(&SweepInput {
        contracts: &contracts_snapshot,
        rules: &rules,
        dispatched: &dispatched,
        auto_open_case_days: auto_open.then_some(thresholds.expiring_soon_days),
    });

    let admins = automation::admin_ids(pool).await?;
    let system_user = admins.first().copied();
    let by_id = |id: Uuid| rows.iter().find(|r| r.id == id);

    for action in actions {
        match action {
            SweepAction::Skip {
                contract_id,
                rule_id,
            } => {
                automation::record_dispatch(pool, contract_id, rule_id, true).await?;
                summary.reminders_skipped += 1;
            }
            SweepAction::Fire {
                contract_id,
                rule_id,
            } => {
                let Some(row) = by_id(contract_id) else {
                    continue;
                };
                let Some(rule) = rules.iter().find(|r| r.id == rule_id) else {
                    continue;
                };
                automation::record_dispatch(pool, contract_id, rule_id, false).await?;
                summary.reminders_fired += 1;
                let recipients = audience(
                    row.case_assigned_employee_id.or(row.assigned_employee_id),
                    &admins,
                );
                let title = format!(
                    "{} · {} {} expires in {} days",
                    row.tenant_name, row.building_name, row.unit_numbers, row.remaining_days
                );
                let body = format!(
                    "{} — contract {} ends {}. Reminder: {}.",
                    row.tenant_name,
                    row.contract_number,
                    fmt_date(row.end_date),
                    rule.label
                );
                if rule.notify_in_app {
                    let (etype, eid) = row
                        .case_id
                        .map(|c| ("renewal_case", c))
                        .unwrap_or(("contract", row.id));
                    let kind = if i64::from(row.remaining_days) <= thresholds.urgent_days {
                        "CONTRACT_EXPIRING_SOON"
                    } else {
                        "RENEWAL_REMINDER"
                    };
                    summary.notifications_created += notify_many(
                        pool,
                        &recipients,
                        kind,
                        &title,
                        Some(&body),
                        etype,
                        eid,
                        Some(&format!("rule:{}:{}", rule.id, row.id)),
                    )
                    .await?;
                }
                if rule.mark_urgent {
                    if let Some(case_id) = row.case_id {
                        renewals::set_urgent(pool, case_id, true).await?;
                    }
                }
                if rule.email_assigned_employee {
                    summary.emails_queued += email_employee(pool, row, system_user).await?;
                }
            }
            SweepAction::OpenCase { contract_id } => {
                let Some(row) = by_id(contract_id) else {
                    continue;
                };
                if renewals::open_for_contract(pool, contract_id)
                    .await?
                    .is_none()
                {
                    let mut tx = pool.begin().await?;
                    let opened_by = system_user.unwrap_or(Uuid::nil());
                    let assigned = row.assigned_employee_id.or(system_user);
                    let case_id = renewals::insert(
                        &mut tx,
                        contract_id,
                        assigned,
                        row.remaining_days <= thresholds.urgent_days as i32,
                        opened_by,
                    )
                    .await?;
                    renewal_db::audit::record(
                        &mut *tx,
                        &renewal_db::audit::AuditEntry {
                            actor_id: actor.map(|a| a.user_id),
                            entity_type: "renewal_case",
                            entity_id: Some(case_id),
                            action: "RENEWAL_STARTED",
                            before: None,
                            after: Some(serde_json::json!({ "by": "expiry sweep", "contractId": contract_id })),
                        },
                    )
                    .await?;
                    tx.commit().await?;
                    summary.cases_opened += 1;
                    let recipients = audience(assigned, &admins);
                    let title = format!(
                        "Renewal case opened: {} · {} {}",
                        row.tenant_name, row.building_name, row.unit_numbers
                    );
                    summary.notifications_created += notify_many(
                        pool,
                        &recipients,
                        "CASE_ASSIGNED",
                        &title,
                        None,
                        "renewal_case",
                        case_id,
                        Some(&format!("case-open:{case_id}")),
                    )
                    .await?;
                }
            }
            SweepAction::Expire { contract_id } => {
                let Some(row) = by_id(contract_id) else {
                    continue;
                };
                let expired = contract_svc::expire_overdue(pool, actor).await?;
                if expired.contains(&contract_id) {
                    summary.contracts_expired += 1;
                    let recipients = audience(
                        row.case_assigned_employee_id.or(row.assigned_employee_id),
                        &admins,
                    );
                    let title = format!(
                        "Contract expired: {} · {} {}",
                        row.tenant_name, row.building_name, row.unit_numbers
                    );
                    summary.notifications_created += notify_many(
                        pool,
                        &recipients,
                        "CONTRACT_EXPIRED",
                        &title,
                        Some(&format!(
                            "{} ended on {} without renewal.",
                            row.contract_number,
                            fmt_date(row.end_date)
                        )),
                        "contract",
                        row.id,
                        Some(&format!("expired:{}", row.id)),
                    )
                    .await?;
                }
            }
        }
    }

    // Follow-ups due today / overdue (spec §12, §15) — once per user per day.
    for f in automation::due_follow_ups(pool).await? {
        let recipients = audience(f.assigned_employee_id, &admins);
        let overdue = f.days_until_due < 0;
        let kind = if overdue {
            "OVERDUE_FOLLOW_UP"
        } else {
            "FOLLOW_UP_DUE_TODAY"
        };
        let title = if overdue {
            format!(
                "Overdue follow-up: {} ({}) — {} days late",
                f.tenant_name, f.contract_number, -f.days_until_due
            )
        } else {
            format!(
                "Follow-up due today: {} ({})",
                f.tenant_name, f.contract_number
            )
        };
        summary.follow_up_alerts += notify_many(
            pool,
            &recipients,
            kind,
            &title,
            None,
            "renewal_case",
            f.case_id,
            Some(&format!("follow-up:{}:{}", f.id, today)),
        )
        .await?;
    }

    // Notices still pending on expiring contracts; tenants silent for 7+ days after a notice.
    for c in automation::pending_cases(pool).await? {
        let recipients = audience(c.assigned_employee_id, &admins);
        if matches!(c.notice_status.as_str(), "PENDING" | "DRAFT")
            && c.remaining_days <= thresholds.expiring_soon_days as i32
        {
            let title = format!(
                "Renewal notice pending: {} ({}) — {} days left",
                c.tenant_name, c.contract_number, c.remaining_days
            );
            summary.notice_alerts += notify_many(
                pool,
                &recipients,
                "RENEWAL_NOTICE_PENDING",
                &title,
                None,
                "renewal_case",
                c.id,
                Some(&format!("notice-pending:{}:{}", c.id, today)),
            )
            .await?;
        }
        if matches!(
            c.status.as_str(),
            "NOTICE_SENT" | "WAITING_FOR_TENANT_RESPONSE"
        ) && c.days_since_notice.unwrap_or(0) >= 7
        {
            let title = format!(
                "No tenant response: {} ({}) — {} days since notice",
                c.tenant_name,
                c.contract_number,
                c.days_since_notice.unwrap_or(0)
            );
            summary.response_alerts += notify_many(
                pool,
                &recipients,
                "TENANT_RESPONSE_PENDING",
                &title,
                None,
                "renewal_case",
                c.id,
                Some(&format!("response-pending:{}:{}", c.id, today)),
            )
            .await?;
        }
    }

    // Rent cheques: N days before the cheque date, on the day, and once when overdue.
    summary.cheque_alerts = crate::cheques::send_reminders(pool, today, &admins).await?;

    renewal_db::worker_status::record_sweep(
        pool,
        &serde_json::to_value(&summary).unwrap_or_default(),
    )
    .await?;
    Ok(summary)
}

/// Queues the INTERNAL_REMINDER template to the assigned employee (or Admins).
async fn email_employee(
    pool: &PgPool,
    row: &SweepRow,
    system_user: Option<Uuid>,
) -> ServiceResult<usize> {
    let Some(user_id) = row.case_assigned_employee_id.or(row.assigned_employee_id) else {
        return Ok(0);
    };
    let Some(user) = users::find_by_id(pool, user_id).await? else {
        return Ok(0);
    };
    let Some(sender) = system_user else {
        return Ok(0);
    };
    let contract = match contracts::find(pool, row.id).await? {
        Some(c) => c,
        None => return Ok(0),
    };
    let tpl = templates::get(pool, "INTERNAL_REMINDER").await?;
    let ctx = serde_json::json!({
        "TenantName": contract.tenant_name,
        "ContactPerson": contract.tenant_contact.clone().unwrap_or_else(|| contract.tenant_name.clone()),
        "BuildingName": contract.building_name,
        "UnitNumber": contract.unit_numbers,
        "ContractNumber": contract.contract_number,
        "ContractStartDate": contract.start_date.format("%d %B %Y").to_string(),
        "ContractEndDate": contract.end_date.format("%d %B %Y").to_string(),
        "RemainingDays": contract.remaining_days.to_string(),
        "ProposedRenewalPeriod": "",
        "OtherRenewalTerms": "",
        "ResponsibleEmployee": user.name,
        "RenewalStatus": contract.renewal_status.clone().unwrap_or_else(|| "NOT_STARTED".into()),
        "NoticeStatus": contract.notice_status.clone().unwrap_or_else(|| "PENDING".into()),
        "CompanyName": settings::get::<String>(pool, settings::ORG_NAME).await?.unwrap_or_default(),
        "Today": renewal_db::today(pool).await?.format("%d %B %Y").to_string(),
    });
    let subject = templates::render(&tpl.subject, &ctx)?;
    let body = templates::render(&tpl.body_text, &ctx)?;
    emails::queue_system(
        pool,
        sender,
        emails::QueueEmail {
            email_type: "INTERNAL_REMINDER".into(),
            tenant_id: Some(contract.tenant_id),
            contract_id: Some(contract.id),
            case_id: row.case_id,
            to: vec![user.email],
            cc: vec![],
            subject,
            body_text: body,
            attachment_document_ids: vec![],
        },
    )
    .await?;
    Ok(1)
}
