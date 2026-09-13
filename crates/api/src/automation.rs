//! Phase 6 wire types: notifications, reminder schedule, organisation settings.

use renewal_core::Thresholds;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Notification {
    pub id: String,
    /// CHEQUE_DUE | CHEQUE_OVERDUE |
    /// CONTRACT_EXPIRING_SOON | RENEWAL_NOTICE_PENDING | TENANT_RESPONSE_PENDING | FOLLOW_UP_DUE_TODAY |
    /// OVERDUE_FOLLOW_UP | CONTRACT_EXPIRED | RENEWAL_REMINDER | RENEWAL_COMPLETED | CASE_ASSIGNED
    pub kind: String,
    pub title: String,
    pub body: Option<String>,
    /// tenant | unit | contract | renewal_case
    pub entity_type: String,
    pub entity_id: String,
    pub created_at: String,
    pub read_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct UnreadCount {
    pub unread: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ReminderRuleDto {
    pub id: Option<String>,
    pub days_before: i32,
    pub label: String,
    pub notify_in_app: bool,
    pub email_assigned_employee: bool,
    pub mark_urgent: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct LetterheadDto {
    pub company_name: String,
    pub address_lines: Vec<String>,
    pub phone: String,
    pub email: String,
    pub footer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct OrgSettingsDto {
    pub org_name: String,
    pub timezone: String,
    pub thresholds: Thresholds,
    pub auto_open_case: bool,
    pub completed_window_days: i32,
    /// Days before a rent cheque's date to remind about the deposit (0 = only on the day).
    #[serde(default = "default_cheque_reminder_days")]
    pub cheque_reminder_days: i32,
    pub letterhead: LetterheadDto,
}

fn default_cheque_reminder_days() -> i32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SweepSummaryDto {
    pub contracts_checked: i64,
    pub reminders_fired: i64,
    pub reminders_skipped: i64,
    pub cases_opened: i64,
    pub contracts_expired: i64,
    pub notifications_created: i64,
    pub emails_queued: i64,
    pub follow_up_alerts: i64,
    pub notice_alerts: i64,
    pub response_alerts: i64,
    #[serde(default)]
    pub cheque_alerts: i64,
}

/// Spec §19: user, date/time, action, previous value, new value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AuditEntry {
    pub id: i64,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub action: String,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    pub created_at: String,
}
