//! Main dashboard (spec §1) and global search.

use renewal_core::Capability;
use renewal_db::contracts::{self, BandCount, ContractFilter, ContractRow};
use renewal_db::dashboard::{self, DashboardCounts, SearchHit};
use renewal_db::follow_ups::{self, FollowUpCounts};
use renewal_db::paging::ListQuery;
use renewal_db::renewals::{self, CaseFilter, CaseRow};
use renewal_db::{settings, users, PgPool};

use crate::error::ServiceResult;
use crate::session::Session;

pub const DEFAULT_COMPLETED_WINDOW_DAYS: i32 = 90;
const TABLE_LIMIT: i64 = 10;

pub struct DashboardData {
    pub counts: DashboardCounts,
    pub bands: Vec<BandCount>,
    pub urgent_renewals: Vec<ContractRow>,
    pub upcoming_expiries: Vec<ContractRow>,
    pub pending_tenant_responses: Vec<ContractRow>,
    pub notices_pending: Vec<ContractRow>,
    pub recently_completed: Vec<CaseRow>,
    pub follow_ups: FollowUpCounts,
    pub completed_window_days: i32,
}

fn top(sort: &str, dir: &str) -> ListQuery {
    ListQuery::new(
        None,
        Some(1),
        Some(TABLE_LIMIT),
        Some(sort.into()),
        Some(dir),
    )
}

pub async fn load(pool: &PgPool, caller: &Session) -> ServiceResult<DashboardData> {
    caller.require(Capability::ViewDashboard)?;
    let window: i32 = settings::get(pool, "dashboard.completedWindowDays")
        .await?
        .unwrap_or(DEFAULT_COMPLETED_WINDOW_DAYS);
    let counts = dashboard::counts(pool, window).await?;
    let bands = contracts::band_counts(pool).await?;
    let can_contracts = caller.role.allows(Capability::ViewContracts);
    let can_renewals = caller.role.allows(Capability::ViewRenewals);

    let active = Some(vec!["ACTIVE".to_owned()]);
    let urgent_renewals = if can_contracts {
        contracts::list(
            pool,
            &ContractFilter {
                status: active.clone(),
                urgent: Some(true),
                ..Default::default()
            },
            &top("remaining_days", "asc"),
        )
        .await?
        .items
    } else {
        vec![]
    };
    let upcoming_expiries = if can_contracts {
        contracts::list(
            pool,
            &ContractFilter {
                status: active.clone(),
                expiring_soon: Some(true),
                urgent: Some(false),
                ..Default::default()
            },
            &top("end_date", "asc"),
        )
        .await?
        .items
    } else {
        vec![]
    };
    let pending_tenant_responses = if can_renewals {
        contracts::list(
            pool,
            &ContractFilter {
                status: active.clone(),
                renewal_status: Some(vec![
                    "NOTICE_SENT".into(),
                    "WAITING_FOR_TENANT_RESPONSE".into(),
                ]),
                ..Default::default()
            },
            &top("remaining_days", "asc"),
        )
        .await?
        .items
    } else {
        vec![]
    };
    let notices_pending = if can_renewals {
        contracts::list(
            pool,
            &ContractFilter {
                status: active,
                notice_status: Some(vec!["PENDING".into(), "DRAFT".into()]),
                ..Default::default()
            },
            &top("remaining_days", "asc"),
        )
        .await?
        .items
    } else {
        vec![]
    };
    let recently_completed = if can_renewals {
        renewals::list(
            pool,
            &CaseFilter {
                completed_within_days: Some(window),
                ..Default::default()
            },
            &top("closed_at", "desc"),
        )
        .await?
        .items
    } else {
        vec![]
    };
    let follow_ups = if caller.role.allows(Capability::ViewFollowUps) {
        follow_ups::counts(pool, None).await?
    } else {
        FollowUpCounts {
            today: 0,
            overdue: 0,
            upcoming: 0,
        }
    };
    Ok(DashboardData {
        counts,
        bands,
        urgent_renewals,
        upcoming_expiries,
        pending_tenant_responses,
        notices_pending,
        recently_completed,
        follow_ups,
        completed_window_days: window,
    })
}

pub async fn search(pool: &PgPool, caller: &Session, term: &str) -> ServiceResult<Vec<SearchHit>> {
    caller.require(Capability::ViewDashboard)?;
    let term = term.trim();
    if term.chars().count() < 2 {
        return Ok(vec![]);
    }
    let hits = dashboard::search(pool, term).await?;
    Ok(hits
        .into_iter()
        .filter(|h| match h.kind.as_str() {
            "building" => caller.role.allows(Capability::ViewBuildings),
            "unit" => caller.role.allows(Capability::ViewUnits),
            "tenant" => caller.role.allows(Capability::ViewTenants),
            "contract" => caller.role.allows(Capability::ViewContracts),
            "occupant" => caller.role.allows(Capability::ViewOccupants),
            "expense" => caller.role.allows(Capability::ViewExpenses),
            _ => false,
        })
        .collect())
}

/// Users who can be assigned to contracts, cases and follow-ups.
pub async fn employees(pool: &PgPool, caller: &Session) -> ServiceResult<Vec<users::UserRow>> {
    caller.require(Capability::ViewDashboard)?;
    Ok(users::list(pool)
        .await?
        .into_iter()
        .filter(|u| u.active)
        .collect())
}
