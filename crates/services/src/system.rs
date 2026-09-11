//! Health and status read by the client's Setup/Settings screens.

use chrono::{DateTime, Duration, Utc};
use renewal_core::Thresholds;
use renewal_db::{settings, users, worker_status, PgPool};

use crate::error::ServiceResult;

/// The scheduler heartbeats every minute; anything older than this is "stale".
pub const SCHEDULER_STALE_AFTER_MINUTES: i64 = 5;

pub struct SchedulerInfo {
    pub version: String,
    pub hostname: String,
    pub last_heartbeat_at: DateTime<Utc>,
    pub last_sweep_at: Option<DateTime<Utc>>,
    pub stale: bool,
}

pub struct SystemInfo {
    pub org_name: String,
    pub timezone: String,
    pub thresholds: Thresholds,
    pub scheduler: Option<SchedulerInfo>,
}

pub async fn users_exist(pool: &PgPool) -> ServiceResult<bool> {
    Ok(users::count(pool).await? > 0)
}

pub async fn info(pool: &PgPool) -> ServiceResult<SystemInfo> {
    let org_name: String = settings::get(pool, settings::ORG_NAME)
        .await?
        .unwrap_or_default();
    let timezone: String = settings::get(pool, settings::ORG_TIMEZONE)
        .await?
        .unwrap_or_else(|| "UTC".into());
    let thresholds: Thresholds = settings::get(pool, settings::EXPIRY_THRESHOLDS)
        .await?
        .unwrap_or_default();
    let scheduler = worker_status::get(pool).await?.map(|w| SchedulerInfo {
        stale: Utc::now() - w.last_heartbeat_at > Duration::minutes(SCHEDULER_STALE_AFTER_MINUTES),
        version: w.version,
        hostname: w.hostname,
        last_heartbeat_at: w.last_heartbeat_at,
        last_sweep_at: w.last_sweep_at,
    });
    Ok(SystemInfo {
        org_name,
        timezone,
        thresholds,
        scheduler,
    })
}
