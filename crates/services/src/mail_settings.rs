//! Settings → Email sending (Admin): the mailbox PropertyPilot sends from, kept in the
//! `settings` table so it can be configured from the app instead of the server's `.env`.
//! The password is stored with the rest of the configuration (the database is already
//! the holder of every secret) and is never returned to clients.

use std::sync::Arc;

use renewal_core::Capability;
use renewal_db::{settings, PgPool};
use serde::{Deserialize, Serialize};

use crate::audit_log::{self, NONE};
use crate::error::{ServiceError, ServiceResult};
use crate::providers::{
    ImapSentConfig, LogMail, MailError, MailProvider, OutboundMail, SendReceipt, SmtpConfig,
    SmtpMail,
};
use crate::session::Session;

pub const KEY: &str = "mail.config";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MailConfig {
    /// `log` (nothing is sent, messages are marked sent) or `smtp`.
    pub provider: String,
    pub from_name: String,
    pub from_address: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    /// `starttls` (usually port 587) or `ssl` (implicit TLS, usually 465).
    pub smtp_security: String,
    pub smtp_username: String,
    #[serde(default)]
    pub smtp_password: String,
    /// Also file each sent message in the mailbox's Sent folder over IMAP.
    pub imap_enabled: bool,
    pub imap_host: String,
    pub imap_port: u16,
    /// Empty = detect the server's `\Sent` folder.
    pub imap_sent_folder: String,
}

/// What the client sees: everything except the password.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailConfigView {
    pub configured: bool,
    pub provider: String,
    pub from_name: String,
    pub from_address: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_security: String,
    pub smtp_username: String,
    pub has_password: bool,
    pub imap_enabled: bool,
    pub imap_host: String,
    pub imap_port: u16,
    pub imap_sent_folder: String,
}

/// What the client sends: `smtp_password = None` keeps the stored one.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailConfigInput {
    pub provider: String,
    pub from_name: String,
    pub from_address: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_security: String,
    pub smtp_username: String,
    pub smtp_password: Option<String>,
    pub imap_enabled: bool,
    pub imap_host: String,
    pub imap_port: u16,
    pub imap_sent_folder: String,
}

pub async fn load(pool: &PgPool) -> ServiceResult<Option<MailConfig>> {
    Ok(settings::get(pool, KEY).await?)
}

fn view_of(cfg: Option<MailConfig>) -> MailConfigView {
    match cfg {
        Some(c) => MailConfigView {
            configured: true,
            provider: c.provider,
            from_name: c.from_name,
            from_address: c.from_address,
            smtp_host: c.smtp_host,
            smtp_port: c.smtp_port,
            smtp_security: c.smtp_security,
            smtp_username: c.smtp_username,
            has_password: !c.smtp_password.is_empty(),
            imap_enabled: c.imap_enabled,
            imap_host: c.imap_host,
            imap_port: c.imap_port,
            imap_sent_folder: c.imap_sent_folder,
        },
        None => MailConfigView {
            configured: false,
            provider: "smtp".into(),
            from_name: "Leasing Department".into(),
            from_address: String::new(),
            smtp_host: String::new(),
            smtp_port: 587,
            smtp_security: "starttls".into(),
            smtp_username: String::new(),
            has_password: false,
            imap_enabled: false,
            imap_host: String::new(),
            imap_port: 993,
            imap_sent_folder: String::new(),
        },
    }
}

pub async fn view(pool: &PgPool, caller: &Session) -> ServiceResult<MailConfigView> {
    caller.require(Capability::ManageSettings)?;
    Ok(view_of(load(pool).await?))
}

fn validate(cfg: &MailConfig) -> ServiceResult<()> {
    if !matches!(cfg.provider.as_str(), "log" | "smtp") {
        return Err(ServiceError::validation("provider must be log or smtp"));
    }
    let addr = cfg.from_address.trim();
    if addr.is_empty() || !addr.contains('@') {
        return Err(ServiceError::validation(
            "a valid sender address is required",
        ));
    }
    if cfg.provider == "smtp" {
        if cfg.smtp_host.trim().is_empty() {
            return Err(ServiceError::validation("SMTP host is required"));
        }
        if cfg.smtp_port == 0 {
            return Err(ServiceError::validation("SMTP port is required"));
        }
        if !matches!(cfg.smtp_security.as_str(), "starttls" | "ssl") {
            return Err(ServiceError::validation("security must be starttls or ssl"));
        }
        if cfg.imap_enabled && cfg.imap_host.trim().is_empty() {
            return Err(ServiceError::validation(
                "IMAP host is required to copy to Sent",
            ));
        }
    }
    Ok(())
}

pub async fn save(
    pool: &PgPool,
    caller: &Session,
    input: MailConfigInput,
) -> ServiceResult<MailConfigView> {
    caller.require(Capability::ManageSettings)?;
    let previous = load(pool).await?;
    let password = match input.smtp_password {
        Some(p) if !p.is_empty() => p,
        _ => previous
            .as_ref()
            .map(|c| c.smtp_password.clone())
            .unwrap_or_default(),
    };
    let cfg = MailConfig {
        provider: input.provider.trim().to_ascii_lowercase(),
        from_name: input.from_name.trim().to_owned(),
        from_address: input.from_address.trim().to_owned(),
        smtp_host: input.smtp_host.trim().to_owned(),
        smtp_port: input.smtp_port,
        smtp_security: input.smtp_security.trim().to_ascii_lowercase(),
        smtp_username: input.smtp_username.trim().to_owned(),
        smtp_password: password,
        imap_enabled: input.imap_enabled,
        imap_host: input.imap_host.trim().to_owned(),
        imap_port: input.imap_port,
        imap_sent_folder: input.imap_sent_folder.trim().to_owned(),
    };
    validate(&cfg)?;
    let mut tx = pool.begin().await?;
    settings::set(&mut *tx, KEY, &cfg, Some(caller.user_id)).await?;
    // Audit the change without the secret.
    let redacted = serde_json::json!({
        "provider": cfg.provider, "fromAddress": cfg.from_address, "smtpHost": cfg.smtp_host,
        "smtpPort": cfg.smtp_port, "smtpUsername": cfg.smtp_username, "imapEnabled": cfg.imap_enabled,
    });
    audit_log::log(
        &mut *tx,
        caller,
        "settings",
        caller.user_id,
        "MAIL_SETTINGS_UPDATED",
        NONE,
        Some(&redacted),
    )
    .await?;
    tx.commit().await?;
    Ok(view_of(Some(cfg)))
}

/// Builds the concrete provider for a saved configuration.
pub fn build_provider(cfg: &MailConfig) -> Result<Arc<dyn MailProvider>, MailError> {
    if cfg.provider == "log" {
        return Ok(Arc::new(LogMail));
    }
    let creds = !cfg.smtp_username.is_empty();
    let smtp = SmtpConfig {
        host: cfg.smtp_host.clone(),
        port: cfg.smtp_port,
        username: creds.then(|| cfg.smtp_username.clone()),
        password: creds.then(|| cfg.smtp_password.clone()),
        from_name: cfg.from_name.clone(),
        from_address: cfg.from_address.clone(),
        starttls: cfg.smtp_security != "ssl",
        imap_sent: cfg.imap_enabled.then(|| ImapSentConfig {
            host: cfg.imap_host.clone(),
            port: if cfg.imap_port == 0 {
                993
            } else {
                cfg.imap_port
            },
            username: cfg.smtp_username.clone(),
            password: cfg.smtp_password.clone(),
            folder: (!cfg.imap_sent_folder.is_empty()).then(|| cfg.imap_sent_folder.clone()),
        }),
    };
    Ok(Arc::new(SmtpMail::new(&smtp)?))
}

/// Sends a plain test message through the active provider (not queued, not logged).
pub async fn send_test(
    caller: &Session,
    provider: &Arc<dyn MailProvider>,
    to: &str,
    org_name: &str,
) -> ServiceResult<SendReceipt> {
    caller.require(Capability::ManageSettings)?;
    let to = to.trim();
    if to.is_empty() || !to.contains('@') {
        return Err(ServiceError::validation(
            "enter the address to send the test to",
        ));
    }
    let mail = OutboundMail {
        to: vec![to.to_owned()],
        cc: vec![],
        subject: format!("[{org_name}] PropertyPilot test email"),
        body_html: format!(
            "<p>This is a test message from PropertyPilot.</p><p>If you can read it, email sending is configured correctly for {org_name}.</p>"
        ),
        attachments: vec![],
    };
    provider
        .send(&mail)
        .await
        .map_err(|e| ServiceError::validation(format!("{e}")))
}
