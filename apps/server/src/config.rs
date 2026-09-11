use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub enum MailConfig {
    /// Development: log the message, mark it sent.
    Log,
    Smtp(renewal_services::providers::SmtpConfig),
    Graph(renewal_services::providers::GraphConfig),
}

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    /// Run the in-process scheduler (heartbeat, expiry sweep, email queue). Off for extra API-only replicas.
    pub scheduler: bool,
    /// IANA zone applied to every DB connection so CURRENT_DATE is the organisation's calendar day.
    pub timezone: String,
    pub mail: MailConfig,
    /// Display address shown in the UI as the sending mailbox.
    pub mail_sender: String,
    /// Serve HTTPS directly when TLS_CERT and TLS_KEY (PEM) are set.
    pub tls: Option<TlsConfig>,
}

fn env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

impl Config {
    pub fn from_env() -> anyhow::Result<Config> {
        let database_url = env("DATABASE_URL")
            .ok_or_else(|| anyhow::anyhow!("DATABASE_URL is not set (see .env.example)"))?;
        let bind_addr: SocketAddr = env("BIND_ADDR")
            .unwrap_or_else(|| "127.0.0.1:8787".into())
            .parse()
            .map_err(|e| anyhow::anyhow!("BIND_ADDR is not a valid host:port: {e}"))?;
        let scheduler = env("SCHEDULER")
            .map(|v| !matches!(v.as_str(), "0" | "off" | "false"))
            .unwrap_or(true);
        let timezone = env("ORG_TIMEZONE").unwrap_or_else(|| "Asia/Dubai".into());

        let provider = env("MAIL_PROVIDER")
            .unwrap_or_else(|| "log".into())
            .to_ascii_lowercase();
        let (mail, mail_sender) = match provider.as_str() {
            "smtp" => {
                let cfg = renewal_services::providers::SmtpConfig {
                    host: env("SMTP_HOST").ok_or_else(|| {
                        anyhow::anyhow!("SMTP_HOST is required for MAIL_PROVIDER=smtp")
                    })?,
                    port: env("SMTP_PORT").and_then(|p| p.parse().ok()).unwrap_or(587),
                    username: env("SMTP_USERNAME"),
                    password: env("SMTP_PASSWORD"),
                    from_name: env("MAIL_FROM_NAME").unwrap_or_else(|| "Leasing Department".into()),
                    from_address: env("MAIL_FROM_ADDRESS").ok_or_else(|| {
                        anyhow::anyhow!("MAIL_FROM_ADDRESS is required for MAIL_PROVIDER=smtp")
                    })?,
                    starttls: env("SMTP_STARTTLS")
                        .map(|v| !matches!(v.as_str(), "0" | "off" | "false"))
                        .unwrap_or(true),
                };
                let sender = cfg.from_address.clone();
                (MailConfig::Smtp(cfg), sender)
            }
            "graph" => {
                let cfg = renewal_services::providers::GraphConfig {
                    tenant_id: env("GRAPH_TENANT_ID").ok_or_else(|| {
                        anyhow::anyhow!("GRAPH_TENANT_ID is required for MAIL_PROVIDER=graph")
                    })?,
                    client_id: env("GRAPH_CLIENT_ID")
                        .ok_or_else(|| anyhow::anyhow!("GRAPH_CLIENT_ID is required"))?,
                    client_secret: env("GRAPH_CLIENT_SECRET")
                        .ok_or_else(|| anyhow::anyhow!("GRAPH_CLIENT_SECRET is required"))?,
                    sender: env("GRAPH_SENDER").ok_or_else(|| {
                        anyhow::anyhow!("GRAPH_SENDER (shared mailbox) is required")
                    })?,
                };
                let sender = cfg.sender.clone();
                (MailConfig::Graph(cfg), sender)
            }
            _ => (
                MailConfig::Log,
                env("MAIL_FROM_ADDRESS").unwrap_or_else(|| "log (not sending)".into()),
            ),
        };
        let tls = match (env("TLS_CERT"), env("TLS_KEY")) {
            (Some(cert_path), Some(key_path)) => Some(TlsConfig {
                cert_path,
                key_path,
            }),
            (None, None) => None,
            _ => anyhow::bail!("set both TLS_CERT and TLS_KEY, or neither"),
        };
        Ok(Config {
            database_url,
            bind_addr,
            scheduler,
            timezone,
            mail,
            mail_sender,
            tls,
        })
    }
}
