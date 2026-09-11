use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct OutboundMail {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub body_html: String,
    /// (file name, MIME type, bytes)
    pub attachments: Vec<(String, String, Vec<u8>)>,
}

#[derive(Debug, Clone)]
pub struct SendReceipt {
    /// Provider-side id (Graph message id, SMTP queue id). Stored on `email_messages`.
    pub provider_message_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum MailError {
    #[error("mail provider rejected the message: {0}")]
    Rejected(String),
    #[error("mail provider unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait MailProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn send(&self, mail: &OutboundMail) -> Result<SendReceipt, MailError>;
}

/// Development provider: logs the message and pretends it was sent.
#[derive(Debug, Default)]
pub struct LogMail;

#[async_trait]
impl MailProvider for LogMail {
    fn name(&self) -> &'static str {
        "log"
    }

    async fn send(&self, mail: &OutboundMail) -> Result<SendReceipt, MailError> {
        tracing::info!(to = ?mail.to, cc = ?mail.cc, subject = %mail.subject, attachments = mail.attachments.len(), "LogMail: pretending to send");
        Ok(SendReceipt {
            provider_message_id: Some(format!("log-{}", uuid::Uuid::new_v4())),
        })
    }
}
