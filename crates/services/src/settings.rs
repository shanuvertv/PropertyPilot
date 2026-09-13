//! Admin settings (spec §8 reminder schedule, thresholds, organisation profile).

use renewal_core::{Capability, Thresholds};
use renewal_db::automation::{self, RuleInput, RuleRow};
use renewal_db::{settings, PgPool};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::audit_log;
use crate::error::{ServiceError, ServiceResult};
use crate::pdf::Letterhead;
use crate::session::Session;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgSettings {
    pub org_name: String,
    pub timezone: String,
    pub thresholds: Thresholds,
    pub auto_open_case: bool,
    pub completed_window_days: i32,
    /// Days before a rent cheque's date to remind about the deposit (0 = only on the day).
    pub cheque_reminder_days: i32,
    pub letterhead: Letterhead,
}

pub async fn org(pool: &PgPool) -> ServiceResult<OrgSettings> {
    Ok(OrgSettings {
        org_name: settings::get(pool, settings::ORG_NAME)
            .await?
            .unwrap_or_default(),
        timezone: settings::get(pool, settings::ORG_TIMEZONE)
            .await?
            .unwrap_or_else(|| "UTC".into()),
        thresholds: settings::get(pool, settings::EXPIRY_THRESHOLDS)
            .await?
            .unwrap_or_default(),
        auto_open_case: settings::get(pool, "renewals.autoOpenCase")
            .await?
            .unwrap_or(true),
        completed_window_days: settings::get(pool, "dashboard.completedWindowDays")
            .await?
            .unwrap_or(90),
        cheque_reminder_days: settings::get(pool, crate::cheques::REMINDER_DAYS_KEY)
            .await?
            .unwrap_or(3),
        letterhead: settings::get(pool, "org.letterhead")
            .await?
            .unwrap_or_default(),
    })
}

pub async fn save_org(
    pool: &PgPool,
    caller: &Session,
    input: OrgSettings,
) -> ServiceResult<OrgSettings> {
    caller.require(Capability::ManageSettings)?;
    if input.org_name.trim().is_empty() {
        return Err(ServiceError::validation("organisation name is required"));
    }
    let t = input.thresholds;
    if t.urgent_days < 0 || t.expiring_soon_days < t.urgent_days || t.expiring_soon_days > 365 {
        return Err(ServiceError::validation(
            "thresholds must satisfy 0 ≤ urgent ≤ expiring-soon ≤ 365 days",
        ));
    }
    if !(7..=365).contains(&input.completed_window_days) {
        return Err(ServiceError::validation(
            "the completed-renewals window must be between 7 and 365 days",
        ));
    }
    if !(0..=60).contains(&input.cheque_reminder_days) {
        return Err(ServiceError::validation(
            "the cheque reminder must be between 0 and 60 days before",
        ));
    }
    let before = org(pool).await?;
    let mut tx = pool.begin().await?;
    let by = Some(caller.user_id);
    settings::set(&mut *tx, settings::ORG_NAME, &input.org_name.trim(), by).await?;
    settings::set(&mut *tx, settings::ORG_TIMEZONE, &input.timezone.trim(), by).await?;
    settings::set(&mut *tx, settings::EXPIRY_THRESHOLDS, &input.thresholds, by).await?;
    settings::set(&mut *tx, "renewals.autoOpenCase", &input.auto_open_case, by).await?;
    settings::set(
        &mut *tx,
        crate::cheques::REMINDER_DAYS_KEY,
        &input.cheque_reminder_days,
        by,
    )
    .await?;
    settings::set(
        &mut *tx,
        "dashboard.completedWindowDays",
        &input.completed_window_days,
        by,
    )
    .await?;
    let mut lh = input.letterhead.clone();
    if lh.company_name.trim().is_empty() {
        lh.company_name = input.org_name.trim().to_owned();
    }
    settings::set(&mut *tx, "org.letterhead", &lh, by).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "settings",
        Uuid::nil(),
        "ORG_SETTINGS_UPDATED",
        Some(&before),
        Some(&input),
    )
    .await?;
    tx.commit().await?;
    org(pool).await
}

pub async fn reminder_rules(pool: &PgPool, caller: &Session) -> ServiceResult<Vec<RuleRow>> {
    caller.require(Capability::ManageSettings)?;
    Ok(automation::rules(pool, false).await?)
}

pub struct RuleDraft {
    pub id: Option<Uuid>,
    pub days_before: i32,
    pub label: String,
    pub notify_in_app: bool,
    pub email_assigned_employee: bool,
    pub mark_urgent: bool,
    pub active: bool,
}

pub async fn save_reminder_rules(
    pool: &PgPool,
    caller: &Session,
    items: Vec<RuleDraft>,
) -> ServiceResult<Vec<RuleRow>> {
    caller.require(Capability::ManageSettings)?;
    let mut seen = std::collections::HashSet::new();
    for r in &items {
        if r.days_before < 0 || r.days_before > 730 {
            return Err(ServiceError::validation(
                "days before expiry must be between 0 and 730",
            ));
        }
        if r.active && !seen.insert(r.days_before) {
            return Err(ServiceError::validation(format!(
                "two active rules use {} days",
                r.days_before
            )));
        }
        if r.label.trim().is_empty() {
            return Err(ServiceError::validation("every rule needs a label"));
        }
    }
    let mut tx = pool.begin().await?;
    let before = automation::rules(&mut *tx, false).await?;
    let inputs: Vec<RuleInput<'_>> = items
        .iter()
        .map(|r| RuleInput {
            id: r.id,
            days_before: r.days_before,
            label: r.label.trim(),
            notify_in_app: r.notify_in_app,
            email_assigned_employee: r.email_assigned_employee,
            mark_urgent: r.mark_urgent,
            active: r.active,
        })
        .collect();
    automation::save_rules(&mut tx, &inputs).await?;
    let after = automation::rules(&mut *tx, false).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "settings",
        Uuid::nil(),
        "REMINDER_RULES_UPDATED",
        Some(&before),
        Some(&after),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}
