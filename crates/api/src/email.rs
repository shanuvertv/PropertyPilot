//! Phase 5 wire types: email templates, messages, renewal notices.

use renewal_core::{EmailStatus, NoticeStatus};
use serde::{Deserialize, Serialize};

use crate::master::ListParams;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct EmailTemplate {
    pub id: String,
    pub key: String,
    pub name: String,
    pub subject: String,
    pub body_text: String,
    pub active: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct EmailTemplateInput {
    pub name: String,
    pub subject: String,
    pub body_text: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Placeholder {
    pub name: String,
    pub description: String,
}

/// `POST /api/email-templates/{key}/preview` — renders against a case (or sample data).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PreviewRequest {
    pub case_id: Option<String>,
    pub subject: Option<String>,
    pub body_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Preview {
    pub subject: String,
    pub body_text: String,
    pub body_html: String,
    pub recipient: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct EmailMessage {
    pub id: String,
    pub email_type: String,
    pub tenant_id: Option<String>,
    pub tenant_name: Option<String>,
    pub contract_id: Option<String>,
    pub contract_number: Option<String>,
    pub case_id: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub body_text: String,
    pub status: EmailStatus,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub provider: Option<String>,
    pub sent_by_name: Option<String>,
    pub attachment_count: i64,
    pub queued_at: String,
    pub sent_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ComposeEmailRequest {
    /// Template key (for the log's "Email Type") or `CUSTOM`.
    pub email_type: String,
    pub tenant_id: Option<String>,
    pub contract_id: Option<String>,
    pub case_id: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub body_text: String,
    pub attachment_document_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct EmailListParams {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub sort: Option<String>,
    pub dir: Option<String>,
    pub tenant_id: Option<String>,
    pub contract_id: Option<String>,
    pub case_id: Option<String>,
    pub status: Option<EmailStatus>,
    pub email_type: Option<String>,
}

impl EmailListParams {
    pub fn list(&self) -> ListParams {
        ListParams {
            q: self.q.clone(),
            page: self.page,
            page_size: self.page_size,
            sort: self.sort.clone(),
            dir: self.dir.clone(),
        }
    }
}

// ---------------------------------------------------------------- notices

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Notice {
    pub id: String,
    pub case_id: String,
    pub status: NoticeStatus,
    pub proposed_period: Option<String>,
    pub other_terms: Option<String>,
    pub subject: String,
    pub body_text: String,
    pub recipient: Option<String>,
    pub cc: Vec<String>,
    pub pdf_document_id: Option<String>,
    pub email_message_id: Option<String>,
    pub email_status: Option<EmailStatus>,
    pub sent_by_name: Option<String>,
    pub sent_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// `GET /api/renewals/{id}/notice` — the draft (or a freshly prepared one) plus history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct NoticeWorkspace {
    pub draft: Option<Notice>,
    pub prepared: Preview,
    pub proposed_period: String,
    pub other_terms: String,
    pub history: Vec<Notice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct NoticeDraftInput {
    pub subject: String,
    pub body_text: String,
    pub proposed_period: Option<String>,
    pub other_terms: Option<String>,
    pub recipient: Option<String>,
    pub cc: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SendNoticeRequest {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub email_body_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MailStatus {
    pub provider: String,
    pub sender: String,
    pub queued: i64,
    pub failed: i64,
}
