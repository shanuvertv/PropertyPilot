//! SMTP provider (any mailbox: Microsoft 365 SMTP AUTH, Google Workspace, cPanel, Zoho…)
//! via `lettre`, with an optional copy of each sent message into the mailbox's IMAP
//! Sent folder so the team's mail client shows what PropertyPilot sent.

use async_trait::async_trait;
use lettre::message::{header::ContentType, Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use super::imap_sent::{append_to_sent, ImapSentConfig};
use super::mail::{MailError, MailProvider, OutboundMail, SendReceipt};

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub from_name: String,
    pub from_address: String,
    /// STARTTLS (587) when true; implicit TLS (465) when false.
    pub starttls: bool,
    /// When set, every sent message is also appended to this IMAP account's Sent folder.
    pub imap_sent: Option<ImapSentConfig>,
}

pub struct SmtpMail {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
    imap_sent: Option<ImapSentConfig>,
}

impl SmtpMail {
    pub fn new(cfg: &SmtpConfig) -> Result<Self, MailError> {
        let mut builder = if cfg.starttls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.host)
        }
        .map_err(|e| MailError::Unavailable(e.to_string()))?
        .port(cfg.port);
        if let (Some(u), Some(p)) = (&cfg.username, &cfg.password) {
            builder = builder.credentials(Credentials::new(u.clone(), p.clone()));
        }
        let from: Mailbox = format!("{} <{}>", cfg.from_name, cfg.from_address)
            .parse()
            .map_err(|e| MailError::Unavailable(format!("bad sender address: {e}")))?;
        Ok(Self {
            transport: builder.build(),
            from,
            imap_sent: cfg.imap_sent.clone(),
        })
    }
}

fn mailbox(s: &str) -> Result<Mailbox, MailError> {
    s.parse()
        .map_err(|_| MailError::Rejected(format!("invalid address {s}")))
}

#[async_trait]
impl MailProvider for SmtpMail {
    fn name(&self) -> &'static str {
        "smtp"
    }

    async fn send(&self, mail: &OutboundMail) -> Result<SendReceipt, MailError> {
        let mut msg = Message::builder()
            .from(self.from.clone())
            .subject(&mail.subject);
        for to in &mail.to {
            msg = msg.to(mailbox(to)?);
        }
        for cc in &mail.cc {
            msg = msg.cc(mailbox(cc)?);
        }
        let body = SinglePart::builder()
            .header(ContentType::TEXT_HTML)
            .body(mail.body_html.clone());
        let mut part = MultiPart::mixed().singlepart(body);
        for (name, ct, bytes) in &mail.attachments {
            let content_type = ContentType::parse(ct)
                .unwrap_or(ContentType::parse("application/octet-stream").unwrap());
            part = part.singlepart(Attachment::new(name.clone()).body(bytes.clone(), content_type));
        }
        let email = msg
            .multipart(part)
            .map_err(|e| MailError::Rejected(e.to_string()))?;
        let raw = email.formatted();
        let res = self
            .transport
            .send(email)
            .await
            .map_err(|e| MailError::Unavailable(e.to_string()))?;
        let id = res.message().next().map(|s| s.to_string());
        if let Some(imap) = self.imap_sent.clone() {
            // Best effort, off the async runtime: the message is already on its way.
            let subject = mail.subject.clone();
            tokio::task::spawn_blocking(move || match append_to_sent(&imap, &raw) {
                Ok(folder) => tracing::debug!(%subject, folder, "copied to IMAP Sent folder"),
                Err(e) => {
                    tracing::warn!(%subject, error = %e, "could not copy to IMAP Sent folder")
                }
            });
        }
        Ok(SendReceipt {
            provider_message_id: id,
        })
    }
}
