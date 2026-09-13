//! `renewal-server`: the one process that owns the database.
//!
//! Serves the HTTP API used by the Windows and Android clients and runs the
//! in-process scheduler (heartbeat, daily expiry sweep, email queue).
//! Configuration comes from the environment or a `.env` file next to the
//! executable (see `.env.example`).
//!
//! Modes: plain console (`renewal-server`) or Windows Service (`renewal-server --service`).

mod auth;
mod config;
mod dto;
mod error;
mod ratelimit;
mod routes;
mod scheduler;
mod service;
mod state;

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use renewal_services::providers::{GraphMail, LogMail, MailProvider, SmtpMail};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::config::{Config, MailConfig};
use crate::state::AppState;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn load_env() {
    // `.env` beside the executable wins (service deployments), then the working directory.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let _ = dotenvy::from_path(dir.join(".env"));
        }
    }
    let _ = dotenvy::dotenv();
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,sqlx=warn".into());
    // Services have no console: also append to a log file next to the executable when LOG_DIR is set.
    if let Ok(dir) = std::env::var("LOG_DIR") {
        let appender = tracing_appender::rolling::daily(dir, "renewal-server.log");
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_writer(appender)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
}

/// Starts everything and runs until `shutdown` resolves. Shared by console and service modes.
pub async fn run_server(shutdown: impl Future<Output = ()> + Send + 'static) -> anyhow::Result<()> {
    let config = Config::from_env()?;
    info!(version = VERSION, bind = %config.bind_addr, scheduler = config.scheduler, tls = config.tls.is_some(), "starting renewal-server");

    let pool = renewal_db::connect(&config.database_url, &config.timezone).await?;
    renewal_db::migrate(&pool).await?;
    info!("database ready");

    let mail: Arc<dyn MailProvider> = match &config.mail {
        MailConfig::Log => Arc::new(LogMail),
        MailConfig::Smtp(cfg) => {
            Arc::new(SmtpMail::new(cfg).map_err(|e| anyhow::anyhow!("SMTP: {e}"))?)
        }
        MailConfig::Graph(cfg) => Arc::new(GraphMail::new(cfg.clone())),
    };
    info!(provider = mail.name(), sender = %config.mail_sender, "mail provider ready");
    let state = AppState::new(
        pool.clone(),
        mail,
        config.mail_sender.clone(),
        config.timezone.clone(),
    );
    let app = routes::router(state.clone(), config.web_dir.clone());

    let scheduler = if config.scheduler {
        Some(scheduler::start(state.clone()).await?)
    } else {
        None
    };

    let addr: SocketAddr = config.bind_addr;
    match &config.tls {
        Some(tls) => {
            let rustls =
                axum_server::tls_rustls::RustlsConfig::from_pem_file(&tls.cert_path, &tls.key_path)
                    .await
                    .map_err(|e| anyhow::anyhow!("TLS: cannot load certificate/key: {e}"))?;
            info!("listening on https://{addr}");
            let handle = axum_server::Handle::new();
            let h = handle.clone();
            tokio::spawn(async move {
                shutdown.await;
                h.graceful_shutdown(Some(std::time::Duration::from_secs(10)));
            });
            axum_server::bind_rustls(addr, rustls)
                .handle(handle)
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await?;
        }
        None => {
            let listener = tokio::net::TcpListener::bind(addr).await?;
            info!("listening on http://{}", listener.local_addr()?);
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(shutdown)
            .await?;
        }
    }

    if let Some(mut s) = scheduler {
        let _ = s.shutdown().await;
    }
    pool.close().await;
    info!("stopped");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    load_env();
    init_tracing();
    let service_mode = std::env::args().any(|a| a == "--service");
    #[cfg(windows)]
    if service_mode {
        return service::windows::run();
    }
    if service_mode {
        anyhow::bail!("--service is only supported on Windows");
    }
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run_server(async {
        let _ = tokio::signal::ctrl_c().await;
        info!("shutdown requested");
    }))
}
