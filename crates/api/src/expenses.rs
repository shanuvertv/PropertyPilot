//! Expenses per unit, split equally between the people living there.
//! Amounts on the wire are major units with two decimals (`1234.56`), never minor units.

use serde::{Deserialize, Serialize};

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
    /// People the bill is split between (0 when not split).
    pub split_count: i64,
    /// How many of them have paid.
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
    /// `NONE` | `EQUAL`. `EQUAL` splits evenly between `split_count` people.
    pub split_method: String,
    /// People to split between; omitted or 0 = the unit's number of tenants.
    #[serde(default)]
    pub split_count: Option<i64>,
    pub notes: Option<String>,
}

/// One person's equal share of a split bill; the first shares carry the rounding remainder.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ExpenseShare {
    /// 1-based position.
    pub index: i64,
    pub amount: f64,
    pub settled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ExpenseDetail {
    pub expense: Expense,
    pub shares: Vec<ExpenseShare>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SplitRequest {
    /// People to split between; omitted = the unit's number of tenants.
    #[serde(default)]
    pub split_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SettleRequest {
    /// How many of the people have paid (0 ..= splitCount).
    pub settled_count: i64,
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
