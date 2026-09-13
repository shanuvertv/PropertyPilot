//! Aggregates for the main dashboard (spec §1) and building summaries (spec §2).

use sqlx::{FromRow, PgExecutor};
use uuid::Uuid;

use crate::DbResult;

#[derive(Debug, Clone, FromRow)]
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

/// `completed_window_days` bounds the "Renewals Completed" card (PLAN.md §3.8).
pub async fn counts<'e>(
    ex: impl PgExecutor<'e>,
    completed_window_days: i32,
) -> DbResult<DashboardCounts> {
    sqlx::query_as(
        "SELECT
            (SELECT count(*) FROM buildings WHERE archived_at IS NULL) AS total_buildings,
            (SELECT count(*) FROM units WHERE archived_at IS NULL) AS total_units,
            (SELECT count(*) FROM units WHERE archived_at IS NULL AND status = 'OCCUPIED') AS occupied_units,
            (SELECT count(*) FROM units WHERE archived_at IS NULL AND status = 'VACANT') AS vacant_units,
            (SELECT count(DISTINCT tenant_id) FROM contracts WHERE status = 'ACTIVE') AS active_tenants,
            (SELECT count(*) FROM contracts WHERE status = 'ACTIVE') AS active_contracts,
            (SELECT count(*) FROM v_contract_expiry WHERE expiring_soon) AS expiring_soon,
            (SELECT count(*) FROM renewal_cases WHERE status NOT IN ('RENEWAL_COMPLETED', 'CLOSED'))
              + (SELECT count(*) FROM v_contract_expiry WHERE expiring_soon AND NOT renewal_in_progress) AS renewals_pending,
            (SELECT count(*) FROM renewal_cases WHERE outcome_contract_id IS NOT NULL
               AND closed_at >= now() - make_interval(days => $1)) AS renewals_completed,
            (SELECT count(*) FROM contracts c JOIN v_contract_expiry e ON e.contract_id = c.id
              WHERE c.status = 'EXPIRED' OR (c.status = 'ACTIVE' AND e.band = 'EXPIRED')) AS expired_contracts",
    )
    .bind(completed_window_days)
    .fetch_one(ex)
    .await
}

#[derive(Debug, Clone, FromRow)]
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

pub async fn building_summary<'e>(
    ex: impl PgExecutor<'e>,
    building_id: Uuid,
) -> DbResult<BuildingSummary> {
    sqlx::query_as(
        "SELECT
            (SELECT count(*) FROM units WHERE building_id = $1 AND archived_at IS NULL) AS total_units,
            (SELECT count(*) FROM units WHERE building_id = $1 AND archived_at IS NULL AND status = 'OCCUPIED') AS occupied_units,
            (SELECT count(*) FROM units WHERE building_id = $1 AND archived_at IS NULL AND status = 'VACANT') AS vacant_units,
            (SELECT count(*) FROM units WHERE building_id = $1 AND archived_at IS NULL AND status = 'RESERVED') AS reserved_units,
            (SELECT count(*) FROM units WHERE building_id = $1 AND archived_at IS NULL AND status = 'MAINTENANCE') AS maintenance_units,
            (SELECT count(*) FROM contracts WHERE building_id = $1 AND status = 'ACTIVE') AS active_contracts,
            (SELECT count(*) FROM contracts c JOIN v_contract_expiry e ON e.contract_id = c.id
              WHERE c.building_id = $1 AND e.expiring_soon) AS expiring_soon,
            (SELECT count(*) FROM renewal_cases rc JOIN contracts c ON c.id = rc.contract_id
              WHERE c.building_id = $1 AND rc.status NOT IN ('RENEWAL_COMPLETED', 'CLOSED'))
              + (SELECT count(*) FROM contracts c JOIN v_contract_expiry e ON e.contract_id = c.id
                  WHERE c.building_id = $1 AND e.expiring_soon AND NOT e.renewal_in_progress) AS renewals_pending",
    )
    .bind(building_id)
    .fetch_one(ex)
    .await
}

#[derive(Debug, Clone, FromRow)]
pub struct SearchHit {
    pub kind: String,
    pub id: Uuid,
    pub title: String,
    pub subtitle: String,
}

/// Global search across buildings, units, tenants, contracts, occupants and expenses (top 5 each).
pub async fn search<'e>(ex: impl PgExecutor<'e>, term: &str) -> DbResult<Vec<SearchHit>> {
    let like = format!("%{}%", term.replace('%', "\\%").replace('_', "\\_"));
    sqlx::query_as(
        "(SELECT 'building' AS kind, id, name AS title, code AS subtitle FROM buildings
           WHERE archived_at IS NULL AND (name ILIKE $1 OR code ILIKE $1 OR location ILIKE $1) ORDER BY name LIMIT 5)
         UNION ALL
         (SELECT 'unit', u.id, b.name || ' · ' || u.unit_number, initcap(lower(u.status)) FROM units u JOIN buildings b ON b.id = u.building_id
           WHERE u.archived_at IS NULL AND (u.unit_number ILIKE $1 OR (b.name || ' ' || u.unit_number) ILIKE $1) ORDER BY b.name, u.unit_number LIMIT 5)
         UNION ALL
         (SELECT 'tenant', id, name, COALESCE(contact_person, '') FROM tenants
           WHERE archived_at IS NULL AND (name ILIKE $1 OR contact_person ILIKE $1 OR email ILIKE $1 OR mobile ILIKE $1) ORDER BY name LIMIT 5)
         UNION ALL
         (SELECT 'contract', c.id, c.contract_number, t.name FROM contracts c JOIN tenants t ON t.id = c.tenant_id
           WHERE c.contract_number ILIKE $1 OR t.name ILIKE $1 ORDER BY c.end_date DESC LIMIT 5)
         UNION ALL
         (SELECT 'occupant', o.unit_id, o.full_name, b.name || ' · ' || u.unit_number || COALESCE(' · bed ' || o.bed_label, '')
            FROM occupants o JOIN units u ON u.id = o.unit_id JOIN buildings b ON b.id = u.building_id
           WHERE (o.move_out IS NULL OR o.move_out >= CURRENT_DATE)
             AND (o.full_name ILIKE $1 OR o.id_number ILIKE $1 OR o.phone ILIKE $1 OR o.email ILIKE $1)
           ORDER BY o.full_name LIMIT 5)
         UNION ALL
         (SELECT 'expense', x.id, x.description, b.name || ' · ' || u.unit_number || ' · ' || to_char(x.expense_date, 'DD Mon YYYY')
            FROM expenses x JOIN units u ON u.id = x.unit_id JOIN buildings b ON b.id = u.building_id
           WHERE x.description ILIKE $1 OR x.vendor ILIKE $1 OR x.reference ILIKE $1
           ORDER BY x.expense_date DESC LIMIT 5)",
    )
    .bind(like)
    .fetch_all(ex)
    .await
}
