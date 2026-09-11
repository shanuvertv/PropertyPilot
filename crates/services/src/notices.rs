//! Renewal notices (spec §7 steps 3–4, §9): prepare → edit → draft → PDF → send.

use std::sync::Arc;

use renewal_core::{Capability, RenewalStatus};
use renewal_db::documents::{self, NewDocument};
use renewal_db::emails::{self, NoticeDraft, NoticeRow};
use renewal_db::{renewals, settings, PgPool};
use uuid::Uuid;

use crate::audit_log::{self, NONE};
use crate::emails::{queue, QueueEmail};
use crate::error::{ServiceError, ServiceResult};
use crate::pdf::{self, Letter, Letterhead};
use crate::providers::StorageProvider;
use crate::session::Session;
use crate::templates;

pub struct Prepared {
    pub subject: String,
    pub body_text: String,
    pub recipient: Option<String>,
    pub proposed_period: String,
    pub other_terms: String,
}

/// Spec §7 step 3: auto-populate the notice from the Renewal Notice template.
pub async fn prepare(
    pool: &PgPool,
    caller: &Session,
    case_id: Uuid,
    proposed: Option<&str>,
    terms: Option<&str>,
) -> ServiceResult<Prepared> {
    caller.require(Capability::ViewRenewals)?;
    let case = renewals::find(pool, case_id)
        .await?
        .ok_or(ServiceError::NotFound("renewal case"))?;
    let proposed_period = proposed
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("a further 12 months on the same terms")
        .to_owned();
    let other_terms = terms.map(str::trim).unwrap_or("").to_owned();
    let r = templates::render_for_case(
        pool,
        "RENEWAL_NOTICE",
        &case,
        Some(&proposed_period),
        Some(&other_terms),
    )
    .await?;
    Ok(Prepared {
        subject: r.subject,
        body_text: r.body_text,
        recipient: case.tenant_email,
        proposed_period,
        other_terms,
    })
}

pub async fn list(pool: &PgPool, caller: &Session, case_id: Uuid) -> ServiceResult<Vec<NoticeRow>> {
    caller.require(Capability::ViewRenewals)?;
    Ok(emails::notices_for_case(pool, case_id).await?)
}

pub struct DraftInput {
    pub subject: String,
    pub body_text: String,
    pub proposed_period: Option<String>,
    pub other_terms: Option<String>,
    pub recipient: Option<String>,
    pub cc: Vec<String>,
}

pub async fn save_draft(
    pool: &PgPool,
    caller: &Session,
    case_id: Uuid,
    input: DraftInput,
) -> ServiceResult<NoticeRow> {
    caller.require(Capability::SendNotices)?;
    let subject = input.subject.trim();
    let body = input.body_text.trim();
    if subject.is_empty() || body.is_empty() {
        return Err(ServiceError::validation(
            "subject and letter text are required",
        ));
    }
    let mut tx = pool.begin().await?;
    let case = renewals::find(&mut *tx, case_id)
        .await?
        .ok_or(ServiceError::NotFound("renewal case"))?;
    let status: RenewalStatus = case.status.parse().unwrap_or(RenewalStatus::NotStarted);
    if !status.is_open() {
        return Err(ServiceError::Conflict("this renewal case is closed".into()));
    }
    let id = emails::upsert_draft(
        &mut *tx,
        case_id,
        &NoticeDraft {
            proposed_period: input
                .proposed_period
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty()),
            other_terms: input
                .other_terms
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty()),
            subject,
            body_text: body,
            recipient: input
                .recipient
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty()),
            cc: &input.cc,
        },
        caller.user_id,
    )
    .await?;
    if case.notice_status == "PENDING" || case.notice_status == "NOT_REQUIRED" {
        renewals::set_notice_status(&mut *tx, case_id, "DRAFT").await?;
    }
    let row = emails::find_notice(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("notice"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "renewal_case",
        case_id,
        "NOTICE_DRAFT_SAVED",
        NONE,
        Some(&serde_json::json!({ "noticeId": id, "subject": subject })),
    )
    .await?;
    tx.commit().await?;
    Ok(row)
}

async fn letterhead(pool: &PgPool) -> ServiceResult<Letterhead> {
    Ok(settings::get::<Letterhead>(pool, "org.letterhead")
        .await?
        .unwrap_or_default())
}

/// Renders the draft (or any notice) to a PDF document attached to the case (spec §7 step 3 "Generate PDF").
pub async fn generate_pdf(
    pool: &PgPool,
    storage: &Arc<dyn StorageProvider>,
    caller: &Session,
    case_id: Uuid,
) -> ServiceResult<(NoticeRow, Uuid)> {
    caller.require(Capability::SendNotices)?;
    let notice = emails::draft_for_case(pool, case_id)
        .await?
        .ok_or_else(|| ServiceError::validation("save the notice as a draft first"))?;
    let case = renewals::find(pool, case_id)
        .await?
        .ok_or(ServiceError::NotFound("renewal case"))?;
    let lh = letterhead(pool).await?;
    let today = renewal_db::today(pool).await?;
    let mut recipient_lines = vec![case.tenant_name.clone()];
    if let Some(c) = &case.tenant_contact {
        recipient_lines.push(format!("Attn: {c}"));
    }
    recipient_lines.push(format!(
        "Unit {}, {}",
        case.unit_numbers, case.building_name
    ));
    let letter = Letter {
        letterhead: &lh,
        date: &today.format("%d %B %Y").to_string(),
        reference: &case.contract_number,
        recipient_lines,
        subject: &notice.subject,
        body_text: &notice.body_text,
        // The template text carries its own sign-off ({{ResponsibleEmployee}}, {{CompanyName}}).
        signature_lines: vec![],
    };
    let bytes = pdf::render_letter(&letter)?;
    let key = format!("documents/notice/{}", Uuid::new_v4());
    storage
        .put(&key, "application/pdf", bytes.clone())
        .await
        .map_err(|e| ServiceError::Internal(e.to_string()))?;
    let file_name = format!(
        "Renewal Notice {} {}.pdf",
        case.contract_number,
        today.format("%Y-%m-%d")
    );
    let mut tx = pool.begin().await?;
    let doc_id = documents::insert(
        &mut *tx,
        &NewDocument {
            entity_type: "notice",
            entity_id: notice.id,
            file_name: &file_name,
            content_type: "application/pdf",
            size_bytes: bytes.len() as i64,
            storage_key: &key,
            uploaded_by: caller.user_id,
        },
    )
    .await?;
    emails::set_notice_pdf(&mut *tx, notice.id, doc_id).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "renewal_case",
        case_id,
        "NOTICE_PDF_GENERATED",
        NONE,
        Some(&serde_json::json!({ "noticeId": notice.id, "documentId": doc_id })),
    )
    .await?;
    tx.commit().await?;
    let row = emails::find_notice(pool, notice.id)
        .await?
        .ok_or(ServiceError::NotFound("notice"))?;
    Ok((row, doc_id))
}

pub struct SendInput {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    /// Optional cover text; the letter itself goes as the PDF and also as the body when empty.
    pub email_body_text: Option<String>,
}

/// Spec §7 step 4: send the notice by email with the PDF attached, then update
/// Notice Status = Sent and move the case to Notice Sent.
pub async fn send(
    pool: &PgPool,
    storage: &Arc<dyn StorageProvider>,
    caller: &Session,
    case_id: Uuid,
    input: SendInput,
) -> ServiceResult<NoticeRow> {
    caller.require(Capability::SendNotices)?;
    let (notice, doc_id) = generate_pdf(pool, storage, caller, case_id).await?;
    let case = renewals::find(pool, case_id)
        .await?
        .ok_or(ServiceError::NotFound("renewal case"))?;
    let body = input
        .email_body_text
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| notice.body_text.clone());
    let message = queue(
        pool,
        caller,
        QueueEmail {
            email_type: "RENEWAL_NOTICE".into(),
            tenant_id: Some(case.tenant_id),
            contract_id: Some(case.contract_id),
            case_id: Some(case_id),
            to: input.to.clone(),
            cc: input.cc.clone(),
            subject: notice.subject.clone(),
            body_text: body,
            attachment_document_ids: vec![doc_id],
        },
    )
    .await?;
    let mut tx = pool.begin().await?;
    let recipient = message.to_addresses.join(", ");
    emails::mark_notice_sent(
        &mut *tx,
        notice.id,
        message.id,
        &recipient,
        &message.cc_addresses,
        caller.user_id,
    )
    .await?;
    renewals::set_notice_status(&mut *tx, case_id, "SENT").await?;
    renewals::tick_by_key(&mut *tx, case_id, "NOTICE_SENT", caller.user_id).await?;
    let status: RenewalStatus = case.status.parse().unwrap_or(RenewalStatus::NotStarted);
    // Walk the case forward to Notice Sent along the allowed path.
    for next in [RenewalStatus::NoticePending, RenewalStatus::NoticeSent] {
        let current: RenewalStatus = renewals::find(&mut *tx, case_id)
            .await?
            .map(|c| c.status.parse().unwrap_or(status))
            .unwrap_or(status);
        if current == RenewalStatus::NoticeSent || !current.can_transition_to(next) {
            continue;
        }
        renewals::set_status(&mut *tx, case_id, &next.to_string()).await?;
    }
    audit_log::log(
        &mut *tx,
        caller,
        "renewal_case",
        case_id,
        "RENEWAL_NOTICE_SENT",
        Some(&case.status),
        Some(&serde_json::json!({ "noticeId": notice.id, "emailId": message.id, "to": message.to_addresses, "cc": message.cc_addresses, "subject": notice.subject })),
    )
    .await?;
    tx.commit().await?;
    emails::find_notice(pool, notice.id)
        .await?
        .ok_or(ServiceError::NotFound("notice"))
}
