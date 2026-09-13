//! PostgreSQL access for the desktop app and the worker.
//!
//! Queries are written with `sqlx::query_as` (runtime-checked) rather than the
//! compile-time `query!` macros, so the workspace builds without a database
//! being reachable. Integration tests use `#[sqlx::test]` against `DATABASE_URL`.

use std::time::Duration;

pub use sqlx;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
pub use sqlx::{PgConnection, PgPool};

pub mod audit;
pub mod automation;
pub mod buildings;
pub mod contracts;
pub mod dashboard;
pub mod documents;
pub mod emails;
pub mod expenses;
pub mod follow_ups;
pub mod occupants;
pub mod paging;
pub mod renewals;
pub mod sessions;
pub mod settings;
pub mod tenants;
pub mod units;
pub mod users;
pub mod worker_status;

pub type DbResult<T> = Result<T, sqlx::Error>;

/// Embedded migrations from `crates/db/migrations`, applied in order.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Open a small pool. Fails fast (5 s) so a bad DSN is reported at startup.
///
/// `timezone` is applied to every connection so `CURRENT_DATE` (remaining days,
/// follow-up due dates) is evaluated in the organisation's calendar, not UTC.
pub async fn connect(database_url: &str, timezone: &str) -> DbResult<PgPool> {
    let tz_ok = !timezone.is_empty()
        && timezone
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '+'));
    if !tz_ok {
        return Err(sqlx::Error::Configuration(
            format!("invalid timezone {timezone:?}").into(),
        ));
    }
    let options: PgConnectOptions = database_url
        .parse::<PgConnectOptions>()?
        .options([("timezone", timezone)]);
    PgPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
}

/// Apply any pending migrations. Idempotent; safe to run on every connect.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}

/// The organisation's current calendar date (connection time zone).
pub async fn today(pool: &PgPool) -> DbResult<chrono::NaiveDate> {
    sqlx::query_scalar("SELECT CURRENT_DATE")
        .fetch_one(pool)
        .await
}

/// Cheap liveness probe used by the "disconnected" banner.
pub async fn ping(pool: &PgPool) -> DbResult<()> {
    sqlx::query("SELECT 1").execute(pool).await.map(|_| ())
}
