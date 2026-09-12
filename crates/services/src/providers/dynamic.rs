//! Mail provider chosen at runtime from Settings → Email sending (`mail.config` in the
//! `settings` table). Falls back to the provider configured through the environment
//! when nothing has been saved in the app. Rebuilds the concrete provider only when
//! the saved configuration changes.

use std::sync::Arc;

use async_trait::async_trait;
use renewal_db::PgPool;
use tokio::sync::Mutex;

use super::mail::{MailError, MailProvider, OutboundMail, SendReceipt};
use crate::mail_settings::{self, MailConfig};

struct Cached {
    fingerprint: String,
    provider: Arc<dyn MailProvider>,
    sender: String,
}

pub struct DynamicMail {
    pool: PgPool,
    fallback: Arc<dyn MailProvider>,
    fallback_sender: String,
    cache: Mutex<Option<Cached>>,
}

/// What the status endpoint shows: provider name and the sending address.
#[derive(Debug, Clone)]
pub struct MailDescription {
    pub provider: String,
    pub sender: String,
    /// True when the configuration comes from Settings rather than the environment.
    pub from_settings: bool,
}

impl DynamicMail {
    pub fn new(pool: PgPool, fallback: Arc<dyn MailProvider>, fallback_sender: String) -> Self {
        Self {
            pool,
            fallback,
            fallback_sender,
            cache: Mutex::new(None),
        }
    }

    /// The provider for the configuration saved right now (or the fallback).
    async fn resolve(&self) -> Result<(Arc<dyn MailProvider>, String, bool), MailError> {
        let cfg: Option<MailConfig> = mail_settings::load(&self.pool)
            .await
            .map_err(|e| MailError::Unavailable(format!("cannot read mail settings: {e}")))?;
        let Some(cfg) = cfg else {
            return Ok((self.fallback.clone(), self.fallback_sender.clone(), false));
        };
        let fingerprint = serde_json::to_string(&cfg).unwrap_or_default();
        let mut cache = self.cache.lock().await;
        if let Some(c) = cache.as_ref() {
            if c.fingerprint == fingerprint {
                return Ok((c.provider.clone(), c.sender.clone(), true));
            }
        }
        let provider = mail_settings::build_provider(&cfg)?;
        let sender = cfg.from_address.clone();
        *cache = Some(Cached {
            fingerprint,
            provider: provider.clone(),
            sender: sender.clone(),
        });
        Ok((provider, sender, true))
    }

    pub async fn describe(&self) -> MailDescription {
        match self.resolve().await {
            Ok((p, sender, from_settings)) => MailDescription {
                provider: p.name().to_owned(),
                sender,
                from_settings,
            },
            Err(e) => MailDescription {
                provider: format!("misconfigured ({e})"),
                sender: String::new(),
                from_settings: true,
            },
        }
    }
}

#[async_trait]
impl MailProvider for DynamicMail {
    fn name(&self) -> &'static str {
        "settings"
    }

    async fn send(&self, mail: &OutboundMail) -> Result<SendReceipt, MailError> {
        let (provider, _, _) = self.resolve().await?;
        provider.send(mail).await
    }
}
