//! Occupants and expenses per unit (shared accommodation: bills split between occupants).
//! Amounts on the wire are major units with two decimals (`1234.56`), never minor units.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------- occupants

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Occupant {
    pub id: String,
    pub unit_id: String,
    pub unit_number: String,
    pub building_id: String,
    pub building_name: String,
    pub tenant_id: Option<String>,
    pub tenant_name: Option<String>,
    pub full_name: String,
    pub id_number: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub bed_label: Option<String>,
    pub move_in: String,
    pub move_out: Option<String>,
    /// Living there today (no move-out date, or one in the future).
    pub current: bool,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct OccupantInput {
    pub tenant_id: Option<String>,
    pub full_name: String,
    pub id_number: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub bed_label: Option<String>,
    pub move_in: String,
    pub move_out: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MoveOutRequest {
    /// `null` re-activates an occupant who was moved out by mistake.
    pub move_out: Option<String>,
}

// ---------------------------------------------------------------- expenses

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Expense {
    pub id: String,
    pub unit_id: String,
    pub unit_number: String,
    pub building_id: String,
    pub building_name: String,
    pub category: String,
    pub description: String,
    pub amount: f64,
    pub expense_date: String,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub vendor: Option<String>,
    pub reference: Option<String>,
    pub split_method: String,
    pub notes: Option<String>,
    pub share_count: i64,
    pub settled_count: i64,
    pub created_by_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ExpenseInput {
    pub unit_id: String,
    pub category: String,
    pub description: String,
    pub amount: f64,
    pub expense_date: String,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub vendor: Option<String>,
    pub reference: Option<String>,
    /// `NONE` | `EQUAL` | `CUSTOM`. `EQUAL` splits between the occupants present on the expense date.
    pub split_method: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ExpenseShare {
    pub occupant_id: String,
    pub occupant_name: String,
    pub bed_label: Option<String>,
    pub amount: f64,
    pub settled_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ExpenseDetail {
    pub expense: Expense,
    pub shares: Vec<ExpenseShare>,
    /// Occupants living in the unit on the expense date — the candidates for a split.
    pub occupants: Vec<Occupant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ShareInput {
    pub occupant_id: String,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SetSharesRequest {
    pub shares: Vec<ShareInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SettleRequest {
    pub settled: bool,
}

// ---------------------------------------------------------------- dashboard

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MonthPoint {
    /// First day of the month, ISO date.
    pub month: String,
    pub amount: f64,
    pub expense_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct GroupPoint {
    pub id: String,
    pub label: String,
    pub sublabel: Option<String>,
    pub amount: f64,
    pub expense_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CategoryPoint {
    pub category: String,
    pub amount: f64,
    pub expense_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ExpenseSummary {
    pub from: String,
    pub to: String,
    pub total: f64,
    pub expense_count: i64,
    pub this_month: f64,
    pub last_month: f64,
    pub outstanding: f64,
    pub outstanding_shares: i64,
    pub monthly: Vec<MonthPoint>,
    pub by_building: Vec<GroupPoint>,
    pub by_unit: Vec<GroupPoint>,
    pub by_category: Vec<CategoryPoint>,
}
