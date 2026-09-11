//! SMTP fallback provider (Microsoft 365 SMTP AUTH or any relay) via `lettre`.

use async_trait::async_trait;
use lettre::message::{header::ContentType, Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

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
}

pub struct SmtpMail {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
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
        let res = self
            .transport
            .send(email)
            .await
            .map_err(|e| MailError::Unavailable(e.to_string()))?;
        let id = res.message().next().map(|s| s.to_string());
        Ok(SendReceipt {
            provider_message_id: id,
        })
    }
}
