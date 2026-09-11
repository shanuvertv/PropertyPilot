//! Outbound email queue and communication history (spec §10; PLAN.md §6.6).

use std::sync::Arc;

use renewal_core::Capability;
use renewal_db::emails::{self, MessageFilter, MessageRow, NewMessage};
use renewal_db::paging::{ListQuery, PageResult};
use renewal_db::PgPool;
use uuid::Uuid;

use crate::audit_log::{self, NONE};
use crate::error::{ServiceError, ServiceResult};
use crate::providers::{MailError, MailProvider, OutboundMail, StorageProvider};
use crate::session::Session;
use crate::templates;

pub const MAX_ATTEMPTS: i32 = 3;

pub async fn history(
    pool: &PgPool,
    caller: &Session,
    f: &MessageFilter,
    q: &ListQuery,
) -> ServiceResult<PageResult<MessageRow>> {
    caller.require(Capability::ViewRenewals)?;
    Ok(emails::list_messages(pool, f, q).await?)
}

pub async fn get(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<MessageRow> {
    caller.require(Capability::ViewRenewals)?;
    emails::find_message(pool, id)
        .await?
        .ok_or(ServiceError::NotFound("email"))
}

fn clean_addresses(list: &[String]) -> ServiceResult<Vec<String>> {
    let mut out = Vec::new();
    for raw in list {
        for part in raw.split([',', ';']) {
            let a = part.trim();
            if a.is_empty() {
                continue;
            }
            if !a.contains('@') || a.contains(' ') {
                return Err(ServiceError::validation(format!(
                    "{a} is not a valid email address"
                )));
            }
            out.push(a.to_owned());
        }
    }
    Ok(out)
}

pub struct QueueEmail {
    pub email_type: String,
    pub tenant_id: Option<Uuid>,
    pub contract_id: Option<Uuid>,
    pub case_id: Option<Uuid>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub body_text: String,
    /// Existing documents to attach (their bytes are fetched by the sender).
    pub attachment_document_ids: Vec<Uuid>,
}

/// Puts a message on the queue; the scheduler sender delivers it (spec section 7 step 4).
pub async fn queue(pool: &PgPool, caller: &Session, q: QueueEmail) -> ServiceResult<MessageRow> {
    caller.require(Capability::SendNotices)?;
    queue_as(pool, caller.user_id, Some(caller), q).await
}

/// Queue on behalf of the system (sweep reminders); `sender` is recorded as Sent By.
pub async fn queue_system(pool: &PgPool, sender: Uuid, q: QueueEmail) -> ServiceResult<MessageRow> {
    queue_as(pool, sender, None, q).await
}

async fn queue_as(
    pool: &PgPool,
    sender: Uuid,
    caller: Option<&Session>,
    q: QueueEmail,
) -> ServiceResult<MessageRow> {
    let to = clean_addresses(&q.to)?;
    let cc = clean_addresses(&q.cc)?;
    if to.is_empty() {
        return Err(ServiceError::validation(
            "at least one recipient is required",
        ));
    }
    let subject = q.subject.trim();
    let body_text = q.body_text.trim();
    if subject.is_empty() || body_text.is_empty() {
        return Err(ServiceError::validation("subject and message are required"));
    }
    let body_html = templates::text_to_html(body_text);
    let mut tx = pool.begin().await?;
    let id = emails::insert_message(
        &mut *tx,
        &NewMessage {
            email_type: &q.email_type,
            tenant_id: q.tenant_id,
            contract_id: q.contract_id,
            case_id: q.case_id,
            to: &to,
            cc: &cc,
            subject,
            body_text,
            body_html: &body_html,
            sent_by: sender,
        },
    )
    .await?;
    for doc_id in q.attachment_document_ids {
        let doc = renewal_db::documents::find(&mut *tx, doc_id)
            .await?
            .ok_or(ServiceError::NotFound("attachment"))?;
        emails::insert_attachment(
            &mut *tx,
            id,
            Some(doc.id),
            &doc.file_name,
            &doc.content_type,
            &doc.storage_key,
        )
        .await?;
    }
    let row = emails::find_message(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("email"))?;
    let entity = q.case_id.or(q.contract_id).or(q.tenant_id).unwrap_or(id);
    let entity_type = if q.case_id.is_some() {
        "renewal_case"
    } else if q.contract_id.is_some() {
        "contract"
    } else if q.tenant_id.is_some() {
        "tenant"
    } else {
        "email"
    };
    let after =
        serde_json::json!({ "emailId": id, "to": to, "subject": subject, "type": q.email_type });
    match caller {
        Some(c) => {
            audit_log::log(
                &mut *tx,
                c,
                entity_type,
                entity,
                "EMAIL_QUEUED",
                NONE,
                Some(&after),
            )
            .await?
        }
        None => {
            renewal_db::audit::record(
                &mut *tx,
                &renewal_db::audit::AuditEntry {
                    actor_id: None,
                    entity_type,
                    entity_id: Some(entity),
                    action: "EMAIL_QUEUED",
                    before: None,
                    after: Some(after),
                },
            )
            .await?
        }
    }
    tx.commit().await?;
    Ok(row)
}

pub async fn retry(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<MessageRow> {
    caller.require(Capability::SendNotices)?;
    let mut tx = pool.begin().await?;
    let before = emails::find_message(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("email"))?;
    if before.status != "FAILED" {
        return Err(ServiceError::Conflict(
            "only failed emails can be retried".into(),
        ));
    }
    emails::requeue(&mut *tx, id).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "email",
        id,
        "EMAIL_RETRIED",
        Some(&before.last_error),
        NONE,
    )
    .await?;
    tx.commit().await?;
    emails::find_message(pool, id)
        .await?
        .ok_or(ServiceError::NotFound("email"))
}

pub struct SendReport {
    pub sent: usize,
    pub failed: usize,
    pub retried: usize,
}

/// One sender pass: deliver due queued messages through the provider (called by the scheduler).
pub async fn send_pending(
    pool: &PgPool,
    provider: &Arc<dyn MailProvider>,
    storage: &Arc<dyn StorageProvider>,
    limit: i64,
) -> ServiceResult<SendReport> {
    let mut report = SendReport {
        sent: 0,
        failed: 0,
        retried: 0,
    };
    let due = {
        let mut tx = pool.begin().await?;
        let rows = emails::claim_due(&mut tx, limit).await?;
        tx.commit().await?;
        rows
    };
    for msg in due {
        let mut attachments = Vec::new();
        let mut attach_error: Option<String> = None;
        for a in emails::attachments(pool, msg.id).await? {
            match storage.get(&a.storage_key).await {
                Ok(obj) => attachments.push((a.file_name, a.content_type, obj.bytes)),
                Err(e) => attach_error = Some(format!("attachment {}: {e}", a.file_name)),
            }
        }
        let outcome = match attach_error {
            Some(e) => Err(MailError::Rejected(e)),
            None => {
                provider
                    .send(&OutboundMail {
                        to: msg.to_addresses.clone(),
                        cc: msg.cc_addresses.clone(),
                        subject: msg.subject.clone(),
                        body_html: msg.body_html.clone(),
                        attachments,
                    })
                    .await
            }
        };
        let mut tx = pool.begin().await?;
        match outcome {
            Ok(receipt) => {
                emails::mark_sent(
                    &mut *tx,
                    msg.id,
                    provider.name(),
                    receipt.provider_message_id.as_deref(),
                )
                .await?;
                emails::sync_notice_delivery(&mut tx, msg.id, true).await?;
                report.sent += 1;
            }
            Err(e) => {
                // Rejected = the message itself is bad (address, size): no point retrying.
                let give_up = matches!(e, MailError::Rejected(_)) || msg.attempts >= MAX_ATTEMPTS;
                emails::mark_failed(&mut *tx, msg.id, &e.to_string(), give_up).await?;
                if give_up {
                    emails::sync_notice_delivery(&mut tx, msg.id, false).await?;
                    report.failed += 1;
                } else {
                    report.retried += 1;
                }
            }
        }
        tx.commit().await?;
        tracing::info!(email = %msg.id, to = ?msg.to_addresses, "email pass processed");
    }
    Ok(report)
}
