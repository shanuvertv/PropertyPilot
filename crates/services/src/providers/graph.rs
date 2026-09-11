//! Microsoft Graph `sendMail` from a shared mailbox using client-credentials (PLAN.md D6).
//!
//! App registration needs the *application* permission `Mail.Send` with admin consent;
//! restrict it to the leasing mailbox with an Exchange application access policy.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use base64::Engine;
use serde::Deserialize;
use serde_json::json;

use super::mail::{MailError, MailProvider, OutboundMail, SendReceipt};

#[derive(Debug, Clone)]
pub struct GraphConfig {
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
    /// The mailbox the notices go out from, e.g. leasing@company.com.
    pub sender: String,
}

pub struct GraphMail {
    cfg: GraphConfig,
    http: reqwest::Client,
    token: Mutex<Option<(String, Instant)>>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

impl GraphMail {
    pub fn new(cfg: GraphConfig) -> Self {
        Self {
            cfg,
            http: reqwest::Client::new(),
            token: Mutex::new(None),
        }
    }

    async fn token(&self) -> Result<String, MailError> {
        if let Some((t, exp)) = self.token.lock().unwrap().clone() {
            if Instant::now() < exp {
                return Ok(t);
            }
        }
        let url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.cfg.tenant_id
        );
        let res = self
            .http
            .post(url)
            .form(&[
                ("client_id", self.cfg.client_id.as_str()),
                ("client_secret", self.cfg.client_secret.as_str()),
                ("scope", "https://graph.microsoft.com/.default"),
                ("grant_type", "client_credentials"),
            ])
            .send()
            .await
            .map_err(|e| MailError::Unavailable(format!("token request failed: {e}")))?;
        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(MailError::Unavailable(format!(
                "token request rejected: {body}"
            )));
        }
        let tr: TokenResponse = res
            .json()
            .await
            .map_err(|e| MailError::Unavailable(e.to_string()))?;
        let exp = Instant::now() + Duration::from_secs(tr.expires_in.saturating_sub(60));
        *self.token.lock().unwrap() = Some((tr.access_token.clone(), exp));
        Ok(tr.access_token)
    }
}

#[async_trait]
impl MailProvider for GraphMail {
    fn name(&self) -> &'static str {
        "graph"
    }

    async fn send(&self, mail: &OutboundMail) -> Result<SendReceipt, MailError> {
        let token = self.token().await?;
        let recipients = |list: &[String]| -> Vec<serde_json::Value> {
            list.iter()
                .map(|a| json!({ "emailAddress": { "address": a } }))
                .collect()
        };
        let attachments: Vec<serde_json::Value> = mail
            .attachments
            .iter()
            .map(|(name, ct, bytes)| {
                json!({
                    "@odata.type": "#microsoft.graph.fileAttachment",
                    "name": name,
                    "contentType": ct,
                    "contentBytes": base64::engine::general_purpose::STANDARD.encode(bytes),
                })
            })
            .collect();
        let body = json!({
            "message": {
                "subject": mail.subject,
                "body": { "contentType": "HTML", "content": mail.body_html },
                "toRecipients": recipients(&mail.to),
                "ccRecipients": recipients(&mail.cc),
                "attachments": attachments,
            },
            "saveToSentItems": true
        });
        let url = format!(
            "https://graph.microsoft.com/v1.0/users/{}/sendMail",
            self.cfg.sender
        );
        let res = self
            .http
            .post(url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(|e| MailError::Unavailable(format!("sendMail request failed: {e}")))?;
        match res.status().as_u16() {
            202 => Ok(SendReceipt {
                provider_message_id: res
                    .headers()
                    .get("request-id")
                    .and_then(|v| v.to_str().ok())
                    .map(String::from),
            }),
            429 | 500..=599 => Err(MailError::Unavailable(format!(
                "Graph returned {}",
                res.status()
            ))),
            _ => Err(MailError::Rejected(format!(
                "Graph returned {}: {}",
                res.status(),
                res.text().await.unwrap_or_default()
            ))),
        }
    }
}
