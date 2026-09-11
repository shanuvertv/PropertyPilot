use std::sync::Arc;

use renewal_db::PgPool;
use renewal_services::providers::{MailProvider, PgStorage, StorageProvider};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::ratelimit::RateLimiter;

/// Something changed that clients should refetch. Fan-out over SSE (`GET /api/events`).
#[derive(Debug, Clone)]
pub struct LiveEvent {
    /// `notifications` (bell) or `data` (dashboard / lists).
    pub kind: &'static str,
    /// Target one user, or everyone when `None`.
    pub user_id: Option<Uuid>,
}

impl LiveEvent {
    pub fn notifications(user_id: Option<Uuid>) -> Self {
        Self {
            kind: "notifications",
            user_id,
        }
    }
    pub fn data() -> Self {
        Self {
            kind: "data",
            user_id: None,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub storage: Arc<dyn StorageProvider>,
    pub mail: Arc<dyn MailProvider>,
    pub mail_sender: String,
    pub timezone: String,
    pub events: broadcast::Sender<LiveEvent>,
    /// 10 login attempts per 15 minutes per (ip, email).
    pub login_limiter: Arc<RateLimiter>,
}

impl AppState {
    pub fn new(
        pool: PgPool,
        mail: Arc<dyn MailProvider>,
        mail_sender: String,
        timezone: String,
    ) -> Self {
        let storage: Arc<dyn StorageProvider> = Arc::new(PgStorage::new(pool.clone()));
        let (events, _) = broadcast::channel(256);
        let login_limiter = Arc::new(RateLimiter::new(
            10,
            std::time::Duration::from_secs(15 * 60),
        ));
        Self {
            pool,
            storage,
            mail,
            mail_sender,
            timezone,
            events,
            login_limiter,
        }
    }

    pub fn publish(&self, ev: LiveEvent) {
        let _ = self.events.send(ev);
    }
}
