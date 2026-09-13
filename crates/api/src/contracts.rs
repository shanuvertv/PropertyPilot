//! Phase 2–4 wire types: contracts, renewal cases, responses, follow-ups, checklist, dashboard, search.

use renewal_core::{
    Band, ContractStatus, FollowUpStatus, FollowUpType, NoticeStatus, RenewalStatus, TenantResponse,
};
use serde::{Deserialize, Serialize};

use crate::master::{DocumentInfo, ListParams, UnitSummary};

// ---------------------------------------------------------------- contracts (spec §5, §14)

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Contract {
    pub id: String,
    pub contract_number: String,
    pub tenant_id: String,
    pub tenant_name: String,
    pub tenant_contact: Option<String>,
    pub tenant_email: Option<String>,
    pub building_id: String,
    pub building_name: String,
    pub building_code: String,
    pub unit_ids: Vec<String>,
    pub unit_numbers: String,
    /// Tenants and rent per unit on this contract.
    pub unit_terms: Vec<ContractUnitTerms>,
    /// Number of tenants on the whole contract.
    pub occupant_count: i64,
    pub start_date: String,
    pub end_date: String,
    pub duration_months: i32,
    pub rent_terms: Option<String>,
    /// Rent for the whole contract — the sum of the units' rents; null when none recorded.
    pub rent_amount: Option<f64>,
    pub status: ContractStatus,
    pub assigned_employee_id: Option<String>,
    pub assigned_employee_name: Option<String>,
    pub previous_contract_id: Option<String>,
    pub root_contract_id: Option<String>,
    pub renewal_sequence: i32,
    pub notes: Option<String>,
    pub remaining_days: i32,
    pub band: Band,
    pub expiring_soon: bool,
    pub urgent: bool,
    pub renewal_in_progress: bool,
    pub case_id: Option<String>,
    pub renewal_status: Option<RenewalStatus>,
    pub notice_status: Option<NoticeStatus>,
    pub case_assigned_employee_id: Option<String>,
    pub case_assigned_employee_name: Option<String>,
    pub activated_at: Option<String>,
    pub ended_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// What the contract says about one of its units: how many people live there (what its
/// bills are split by) and its rent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ContractUnitTerms {
    pub unit_id: String,
    #[serde(default)]
    pub occupant_count: i64,
    /// Rent for this unit (major units); null when not recorded.
    #[serde(default)]
    pub rent_amount: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ContractInput {
    pub contract_number: String,
    pub tenant_id: String,
    pub building_id: String,
    pub unit_ids: Vec<String>,
    /// Tenants and rent per unit; units not listed get 0 tenants and no rent.
    #[serde(default)]
    pub unit_terms: Vec<ContractUnitTerms>,
    /// ISO date `YYYY-MM-DD`.
    pub start_date: String,
    pub end_date: String,
    pub rent_terms: Option<String>,
    pub assigned_employee_id: Option<String>,
    pub notes: Option<String>,
    /// Create straight into Active (default) or leave as Draft.
    pub activate: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ContractDetail {
    pub contract: Contract,
    pub units: Vec<UnitSummary>,
    /// Original → Renewal 1 → Renewal 2 … (spec §14 timeline).
    pub chain: Vec<Contract>,
    pub documents: Vec<DocumentInfo>,
    pub open_case: Option<RenewalCase>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ContractListParams {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub sort: Option<String>,
    pub dir: Option<String>,
    pub status: Option<ContractStatus>,
    pub band: Option<Band>,
    pub tenant_id: Option<String>,
    pub building_id: Option<String>,
    pub expiring_soon: Option<bool>,
    pub urgent: Option<bool>,
    pub has_open_case: Option<bool>,
    pub renewal_status: Option<RenewalStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TerminateContractRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct NumberSuggestion {
    pub contract_number: String,
}

// ---------------------------------------------------------------- renewal cases (spec §6, §7, §11, §13)

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RenewalCase {
    pub id: String,
    pub contract_id: String,
    pub contract_number: String,
    pub contract_status: ContractStatus,
    pub status: RenewalStatus,
    pub notice_status: NoticeStatus,
    pub assigned_employee_id: Option<String>,
    pub assigned_employee_name: Option<String>,
    pub is_urgent: bool,
    pub latest_response: Option<TenantResponse>,
    pub latest_response_at: Option<String>,
    pub next_follow_up_date: Option<String>,
    pub outcome_contract_id: Option<String>,
    pub outcome_contract_number: Option<String>,
    pub notes: Option<String>,
    pub tenant_id: String,
    pub tenant_name: String,
    pub tenant_contact: Option<String>,
    pub tenant_email: Option<String>,
    pub tenant_mobile: Option<String>,
    pub building_id: String,
    pub building_name: String,
    pub unit_numbers: String,
    pub start_date: String,
    pub end_date: String,
    pub remaining_days: i32,
    pub band: Band,
    pub checklist_total: i64,
    pub checklist_done: i64,
    /// 0–100, spec §13 "Renewal Progress".
    pub progress_percent: i32,
    pub open_follow_ups: i64,
    pub notice_sent_at: Option<String>,
    pub notice_recipient: Option<String>,
    pub opened_at: String,
    pub closed_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RenewalCaseDetail {
    pub case: RenewalCase,
    pub contract: Contract,
    pub responses: Vec<TenantResponseRecord>,
    pub follow_ups: Vec<FollowUp>,
    pub checklist: Vec<ChecklistItem>,
    /// Transitions allowed from the current status (spec §7 step 2).
    pub allowed_transitions: Vec<RenewalStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct StartRenewalRequest {
    pub assigned_employee_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SetRenewalStatusRequest {
    pub status: RenewalStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct AssignRequest {
    pub assigned_employee_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SetNotesRequest {
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TenantResponseRecord {
    pub id: String,
    pub response: TenantResponse,
    pub response_date: String,
    pub notes: Option<String>,
    pub follow_up_date: Option<String>,
    pub recorded_by_name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RecordResponseRequest {
    pub response: TenantResponse,
    pub response_date: String,
    pub notes: Option<String>,
    /// When set, a follow-up is created on this date (spec §11 "Follow-up Date").
    pub follow_up_date: Option<String>,
    pub follow_up_type: Option<FollowUpType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ChecklistItem {
    pub id: String,
    pub key: Option<String>,
    pub label: String,
    pub required: bool,
    pub done: bool,
    pub done_by_name: Option<String>,
    pub done_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SetChecklistItemRequest {
    pub done: bool,
}

/// Admin-editable checklist template (spec §13).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ChecklistTemplateItem {
    pub id: Option<String>,
    pub key: Option<String>,
    pub label: String,
    pub required: bool,
    pub active: bool,
}

/// Completes a renewal: closes the old contract as Renewed and creates the linked new one (spec §14).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CompleteRenewalRequest {
    pub contract_number: Option<String>,
    pub start_date: String,
    pub end_date: String,
    pub rent_terms: Option<String>,
    /// New tenants / rent per unit; omitted = the old contract's figures.
    #[serde(default)]
    pub unit_terms: Option<Vec<ContractUnitTerms>>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CaseListParams {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub sort: Option<String>,
    pub dir: Option<String>,
    pub status: Option<RenewalStatus>,
    pub notice_status: Option<NoticeStatus>,
    pub band: Option<Band>,
    pub open_only: Option<bool>,
    pub assigned_employee_id: Option<String>,
    pub building_id: Option<String>,
    pub latest_response: Option<TenantResponse>,
}

// ---------------------------------------------------------------- follow-ups (spec §12)

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FollowUp {
    pub id: String,
    pub case_id: String,
    pub contract_id: String,
    pub contract_number: String,
    pub tenant_id: String,
    pub tenant_name: String,
    pub building_name: String,
    pub unit_numbers: String,
    pub due_date: String,
    pub days_until_due: i32,
    pub follow_up_type: FollowUpType,
    pub assigned_employee_id: Option<String>,
    pub assigned_employee_name: Option<String>,
    pub notes: Option<String>,
    pub status: FollowUpStatus,
    pub completed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FollowUpInput {
    pub due_date: String,
    pub follow_up_type: FollowUpType,
    pub assigned_employee_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SetFollowUpStatusRequest {
    pub status: FollowUpStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FollowUpListParams {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub sort: Option<String>,
    pub dir: Option<String>,
    /// `today` | `overdue` | `upcoming` | `open` (default) | `all`
    pub scope: Option<String>,
    pub mine: Option<bool>,
    pub follow_up_type: Option<FollowUpType>,
    pub assigned_employee_id: Option<String>,
    pub building_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FollowUpCounts {
    pub today: i64,
    pub overdue: i64,
    pub upcoming: i64,
}

// ---------------------------------------------------------------- dashboard (spec §1) and search

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DashboardCounts {
    pub total_buildings: i64,
    pub total_units: i64,
    pub occupied_units: i64,
    pub vacant_units: i64,
    pub active_tenants: i64,
    pub active_contracts: i64,
    pub expiring_soon: i64,
    pub renewals_pending: i64,
    pub renewals_completed: i64,
    pub expired_contracts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BandCount {
    pub band: Band,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Dashboard {
    pub counts: DashboardCounts,
    pub bands: Vec<BandCount>,
    pub urgent_renewals: Vec<Contract>,
    pub upcoming_expiries: Vec<Contract>,
    pub pending_tenant_responses: Vec<Contract>,
    pub notices_pending: Vec<Contract>,
    pub recently_completed: Vec<RenewalCase>,
    pub follow_ups: FollowUpCounts,
    pub completed_window_days: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum SearchKind {
    Building,
    Unit,
    Tenant,
    Contract,
    Expense,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SearchHit {
    pub kind: SearchKind,
    pub id: String,
    pub title: String,
    pub subtitle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct EmployeeOption {
    pub id: String,
    pub name: String,
    pub role: renewal_core::Role,
}

impl ContractListParams {
    pub fn list(&self) -> ListParams {
        ListParams {
            q: self.q.clone(),
            page: self.page,
            page_size: self.page_size,
            sort: self.sort.clone(),
            dir: self.dir.clone(),
        }
    }
}

impl CaseListParams {
    pub fn list(&self) -> ListParams {
        ListParams {
            q: self.q.clone(),
            page: self.page,
            page_size: self.page_size,
            sort: self.sort.clone(),
            dir: self.dir.clone(),
        }
    }
}

impl FollowUpListParams {
    pub fn list(&self) -> ListParams {
        ListParams {
            q: self.q.clone(),
            page: self.page,
            page_size: self.page_size,
            sort: self.sort.clone(),
            dir: self.dir.clone(),
        }
    }
}
