//! In-process scheduler. Phase 0: heartbeat + housekeeping + a placeholder for the
//! daily expiry sweep (PLAN.md §6.5, implemented in Phase 6 as a pure `core::sweep`
//! planner plus a database adapter, with the org timezone applied to the cron).

use crate::state::LiveEvent;
use renewal_db::{sessions, worker_status};
use renewal_services::{emails, sweep};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, warn};

use crate::state::AppState;
use crate::VERSION;

pub async fn start(state: AppState) -> anyhow::Result<JobScheduler> {
    let pool = state.pool.clone();
    let host = hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".into());

    // First heartbeat immediately so the Settings screen never shows "no scheduler" right after start.
    if let Err(e) = worker_status::heartbeat(&pool, VERSION, &host).await {
        warn!(error = %e, "initial heartbeat failed");
    }

    let sched = JobScheduler::new().await?;

    // Every minute: heartbeat.
    let (p, h) = (pool.clone(), host.clone());
    sched
        .add(Job::new_async("0 * * * * *", move |_id, _l| {
            let (p, h) = (p.clone(), h.clone());
            Box::pin(async move {
                if let Err(e) = worker_status::heartbeat(&p, VERSION, &h).await {
                    warn!(error = %e, "heartbeat failed");
                }
            })
        })?)
        .await?;

    // Daily 00:05 in the organisation's time zone: the expiry sweep (PLAN.md 6.5) + housekeeping.
    let tz: chrono_tz::Tz = state.timezone.parse().unwrap_or(chrono_tz::UTC);
    let st = state.clone();
    sched
        .add(Job::new_async_tz("0 5 0 * * *", tz, move |_id, _l| {
            let st = st.clone();
            Box::pin(async move {
                let purged = sessions::purge_expired(&st.pool).await.unwrap_or_default();
                match sweep::run(&st.pool, None).await {
                    Ok(s) => {
                        info!(summary = %serde_json::to_value(&s).unwrap_or_default(), sessions_purged = purged, "daily sweep ran");
                        st.publish(LiveEvent::notifications(None));
                        st.publish(LiveEvent::data());
                    }
                    Err(e) => warn!(error = %e, "daily sweep failed"),
                }
            })
        })?)
        .await?;

    // Every 30 s: deliver queued emails through the configured provider (PLAN.md 6.6).
    let st = state.clone();
    sched
        .add(Job::new_async("*/30 * * * * *", move |_id, _l| {
            let st = st.clone();
            Box::pin(async move {
                match emails::send_pending(&st.pool, &st.mail, &st.storage, 20).await {
                    Ok(r) if r.sent + r.failed + r.retried > 0 => {
                        info!(
                            sent = r.sent,
                            failed = r.failed,
                            retried = r.retried,
                            "email pass"
                        );
                        st.publish(LiveEvent::data());
                    }
                    Ok(_) => {}
                    Err(e) => warn!(error = %e, "email pass failed"),
                }
            })
        })?)
        .await?;

    sched.start().await?;
    info!(host = %host, "scheduler started");
    Ok(sched)
}
