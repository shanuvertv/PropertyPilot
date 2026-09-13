//! Post-dated rent cheques per contract and their deposit reminders.
//! Amounts on the wire are major units with two decimals (`1234.56`), never minor units.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Cheque {
    pub id: String,
    pub contract_id: String,
    pub contract_number: String,
    pub contract_status: String,
    pub tenant_id: String,
    pub tenant_name: String,
    pub building_id: String,
    pub building_name: String,
    pub unit_numbers: String,
    /// 1st, 2nd … cheque of the contract.
    pub seq: i64,
    pub cheque_number: Option<String>,
    pub bank_name: Option<String>,
    pub amount: f64,
    /// ISO date on the cheque = the day to deposit it.
    pub due_date: String,
    /// `PENDING` | `DEPOSITED` | `CLEARED` | `BOUNCED` | `CANCELLED`.
    pub status: String,
    pub status_changed_at: Option<String>,
    pub notes: Option<String>,
    /// Negative once the cheque date has passed.
    pub days_until_due: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ChequeInput {
    pub cheque_number: Option<String>,
    pub bank_name: Option<String>,
    pub amount: f64,
    pub due_date: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ChequeStatusRequest {
    pub status: String,
}

/// Split the rent into N cheques spread over the contract period.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct GenerateChequesRequest {
    pub count: i64,
    /// Date of the first cheque; omitted = the contract start date.
    #[serde(default)]
    pub first_date: Option<String>,
    /// Months between cheques; omitted = spread evenly over the contract.
    #[serde(default)]
    pub every_months: Option<i64>,
    /// Total to split; omitted = the contract's rent.
    #[serde(default)]
    pub total: Option<f64>,
    #[serde(default)]
    pub bank_name: Option<String>,
    /// Remove the contract's pending cheques first.
    #[serde(default)]
    pub replace_pending: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ChequeListParams {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub sort: Option<String>,
    pub dir: Option<String>,
    pub contract_id: Option<String>,
    pub building_id: Option<String>,
    pub tenant_id: Option<String>,
    /// Comma-separated statuses.
    pub status: Option<String>,
    pub due_from: Option<String>,
    pub due_to: Option<String>,
    pub overdue: Option<bool>,
}

/// Pending cheques at a glance: overdue, due this week / this month, all pending.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ChequeSummary {
    pub overdue_count: i64,
    pub overdue_amount: f64,
    pub due_7_count: i64,
    pub due_7_amount: f64,
    pub due_30_count: i64,
    pub due_30_amount: f64,
    pub pending_count: i64,
    pub pending_amount: f64,
    pub bounced_count: i64,
    /// Days before the cheque date the reminder goes out (Settings).
    pub reminder_days: i64,
}
