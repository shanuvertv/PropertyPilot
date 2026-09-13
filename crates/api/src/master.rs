//! Phase 1 wire types: buildings, units, tenants, documents, paging.

use renewal_core::{Band, RenewalStatus, UnitStatus};
use serde::{Deserialize, Serialize};

/// Paged list envelope used by every list endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

/// Query string accepted by list endpoints (`?q=&page=&pageSize=&sort=&dir=`).
///
/// Filtered list endpoints repeat these five fields instead of `#[serde(flatten)]`-ing
/// this struct: flatten forces query-string numbers through a string path and fails.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ListParams {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub sort: Option<String>,
    pub dir: Option<String>,
}

impl UnitListParams {
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

// ---------------------------------------------------------------- buildings

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Building {
    pub id: String,
    pub name: String,
    pub code: String,
    pub location: Option<String>,
    pub building_type: Option<String>,
    pub notes: Option<String>,
    pub total_units: i64,
    pub occupied_units: i64,
    pub vacant_units: i64,
    pub reserved_units: i64,
    pub maintenance_units: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BuildingInput {
    pub name: String,
    pub code: String,
    pub location: Option<String>,
    pub building_type: Option<String>,
    pub notes: Option<String>,
}

/// Spec §2 "Building Summary Dashboard".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BuildingSummary {
    pub total_units: i64,
    pub occupied_units: i64,
    pub vacant_units: i64,
    pub reserved_units: i64,
    pub maintenance_units: i64,
    pub active_contracts: i64,
    pub expiring_soon: i64,
    pub renewals_pending: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BuildingDetail {
    pub building: Building,
    pub summary: BuildingSummary,
    pub units: Vec<UnitSummary>,
    pub documents: Vec<DocumentInfo>,
}

// ---------------------------------------------------------------- units

/// One row of the Unit-Wise Summary (spec §3, §16). Contract fields are null for vacant units.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct UnitSummary {
    pub id: String,
    pub building_id: String,
    pub building_name: String,
    pub building_code: String,
    pub unit_number: String,
    pub floor: Option<String>,
    pub unit_type: Option<String>,
    pub status: UnitStatus,
    /// Number of tenants on the unit's active contract (what its bills are split by); 0 when vacant.
    pub occupant_count: i64,
    pub notes: Option<String>,
    pub contract_id: Option<String>,
    pub contract_number: Option<String>,
    /// Rent on the active contract (the contract may cover several units).
    pub rent_amount: Option<f64>,
    pub tenant_id: Option<String>,
    pub tenant_name: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub remaining_days: Option<i32>,
    pub band: Option<Band>,
    pub expiring_soon: bool,
    pub urgent: bool,
    pub case_id: Option<String>,
    pub renewal_status: Option<RenewalStatus>,
    pub assigned_employee_id: Option<String>,
    pub assigned_employee_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct UnitInput {
    pub building_id: String,
    pub unit_number: String,
    pub floor: Option<String>,
    pub unit_type: Option<String>,
    pub status: UnitStatus,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SetUnitStatusRequest {
    pub status: UnitStatus,
}

/// Filters for `GET /api/units` on top of [`ListParams`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct UnitListParams {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub sort: Option<String>,
    pub dir: Option<String>,
    pub building_id: Option<String>,
    pub status: Option<UnitStatus>,
    pub band: Option<Band>,
    pub renewal_status: Option<RenewalStatus>,
    pub tenant_id: Option<String>,
    pub expiring_soon: Option<bool>,
}

// ---------------------------------------------------------------- tenants

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub contact_person: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub alt_contact: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub active_contracts: i64,
    pub current_units: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantListParams {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub sort: Option<String>,
    pub dir: Option<String>,
    pub building_id: Option<String>,
    pub active: Option<bool>,
}

impl TenantListParams {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TenantInput {
    pub name: String,
    pub contact_person: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub alt_contact: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
}

// ---------------------------------------------------------------- documents

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum DocumentEntity {
    Building,
    Tenant,
    Contract,
    Notice,
    Expense,
}

impl DocumentEntity {
    pub fn as_str(self) -> &'static str {
        match self {
            DocumentEntity::Building => "building",
            DocumentEntity::Tenant => "tenant",
            DocumentEntity::Contract => "contract",
            DocumentEntity::Notice => "notice",
            DocumentEntity::Expense => "expense",
        }
    }
    pub fn parse(s: &str) -> Option<DocumentEntity> {
        match s {
            "building" => Some(DocumentEntity::Building),
            "tenant" => Some(DocumentEntity::Tenant),
            "contract" => Some(DocumentEntity::Contract),
            "notice" => Some(DocumentEntity::Notice),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DocumentInfo {
    pub id: String,
    pub entity_type: DocumentEntity,
    pub entity_id: String,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub uploaded_by_name: Option<String>,
    pub created_at: String,
}
