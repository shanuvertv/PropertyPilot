//! Phase 5 routes: email templates, communication log/compose, renewal notices.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use renewal_api::*;
use renewal_db::emails::MessageFilter;
use renewal_services::emails::{self, QueueEmail};
use renewal_services::notices::{self, DraftInput, SendInput};
use renewal_services::{renewals, templates};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::dto;
use crate::error::ApiFailure;
use crate::routes::master::list_query;
use crate::state::AppState;

// ---------------------------------------------------------------- templates

pub async fn list_templates(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<Vec<EmailTemplate>>, ApiFailure> {
    Ok(Json(
        templates::list(&state.pool, &caller)
            .await?
            .into_iter()
            .map(dto::email_template)
            .collect(),
    ))
}

pub async fn placeholders(CurrentUser(_c): CurrentUser) -> Json<Vec<Placeholder>> {
    Json(
        templates::placeholder_help()
            .into_iter()
            .map(|(name, description)| Placeholder { name, description })
            .collect(),
    )
}

pub async fn update_template(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(key): Path<String>,
    Json(input): Json<EmailTemplateInput>,
) -> Result<Json<EmailTemplate>, ApiFailure> {
    let row = templates::update(
        &state.pool,
        &caller,
        &key,
        &input.name,
        &input.subject,
        &input.body_text,
        input.active,
    )
    .await?;
    Ok(Json(dto::email_template(row)))
}

/// Renders a template (or ad-hoc subject/body) against a case, or sample data when no case is given.
pub async fn preview_template(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(key): Path<String>,
    Json(req): Json<PreviewRequest>,
) -> Result<Json<Preview>, ApiFailure> {
    caller.require(renewal_core::Capability::ViewRenewals)?;
    let tpl = templates::get(&state.pool, &key).await?;
    let subject_t = req.subject.unwrap_or(tpl.subject);
    let body_t = req.body_text.unwrap_or(tpl.body_text);
    let (ctx, recipient) = match dto::uuid_opt(&req.case_id, "case")? {
        Some(cid) => {
            let case = renewals::get(&state.pool, &caller, cid).await?;
            let ctx = templates::context_for_case(&state.pool, &case, None, None).await?;
            (ctx, case.tenant_email)
        }
        None => (templates::sample_context(), None),
    };
    let subject = templates::render(&subject_t, &ctx)?;
    let body_text = templates::render(&body_t, &ctx)?;
    Ok(Json(Preview {
        body_html: templates::text_to_html(&body_text),
        subject,
        body_text,
        recipient,
    }))
}

// ---------------------------------------------------------------- messages

pub async fn list_messages(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(p): Query<EmailListParams>,
) -> Result<Json<Page<EmailMessage>>, ApiFailure> {
    let q = list_query(&p.list());
    let f = MessageFilter {
        tenant_id: dto::uuid_opt(&p.tenant_id, "tenant")?,
        contract_id: dto::uuid_opt(&p.contract_id, "contract")?,
        case_id: dto::uuid_opt(&p.case_id, "case")?,
        status: p.status.map(|s| s.to_string()),
        email_type: p.email_type,
    };
    let res = emails::history(&state.pool, &caller, &f, &q).await?;
    Ok(Json(dto::page(
        res,
        q.page,
        q.page_size,
        dto::email_message,
    )))
}

pub async fn get_message(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<EmailMessage>, ApiFailure> {
    Ok(Json(dto::email_message(
        emails::get(&state.pool, &caller, id).await?,
    )))
}

pub async fn compose(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Json(req): Json<ComposeEmailRequest>,
) -> Result<(StatusCode, Json<EmailMessage>), ApiFailure> {
    let row = emails::queue(
        &state.pool,
        &caller,
        QueueEmail {
            email_type: if req.email_type.trim().is_empty() {
                "CUSTOM".into()
            } else {
                req.email_type
            },
            tenant_id: dto::uuid_opt(&req.tenant_id, "tenant")?,
            contract_id: dto::uuid_opt(&req.contract_id, "contract")?,
            case_id: dto::uuid_opt(&req.case_id, "case")?,
            to: req.to,
            cc: req.cc,
            subject: req.subject,
            body_text: req.body_text,
            attachment_document_ids: req
                .attachment_document_ids
                .iter()
                .map(|d| dto::uuid(d, "attachment"))
                .collect::<Result<Vec<_>, _>>()?,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(dto::email_message(row))))
}

pub async fn retry(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<EmailMessage>, ApiFailure> {
    Ok(Json(dto::email_message(
        emails::retry(&state.pool, &caller, id).await?,
    )))
}

pub async fn mail_status(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<MailStatus>, ApiFailure> {
    caller.require(renewal_core::Capability::ManageSettings)?;
    let (queued, failed): (i64, i64) = renewal_db::sqlx::query_as(
        "SELECT count(*) FILTER (WHERE status = 'QUEUED'), count(*) FILTER (WHERE status = 'FAILED') FROM email_messages",
    )
    .fetch_one(&state.pool)
    .await
    .map_err(renewal_services::ServiceError::Db)?;
    Ok(Json(MailStatus {
        provider: state.mail.name().to_owned(),
        sender: state.mail_sender.clone(),
        queued,
        failed,
    }))
}

// ---------------------------------------------------------------- notices

pub async fn notice_workspace(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(case_id): Path<Uuid>,
) -> Result<Json<NoticeWorkspace>, ApiFailure> {
    let history = notices::list(&state.pool, &caller, case_id).await?;
    let draft = history.iter().find(|n| n.status == "DRAFT").cloned();
    let (proposed, terms) = draft
        .as_ref()
        .map(|d| (d.proposed_period.clone(), d.other_terms.clone()))
        .unwrap_or((None, None));
    let prepared = notices::prepare(
        &state.pool,
        &caller,
        case_id,
        proposed.as_deref(),
        terms.as_deref(),
    )
    .await?;
    Ok(Json(NoticeWorkspace {
        draft: draft.map(dto::notice),
        prepared: Preview {
            body_html: templates::text_to_html(&prepared.body_text),
            subject: prepared.subject,
            body_text: prepared.body_text,
            recipient: prepared.recipient,
        },
        proposed_period: prepared.proposed_period,
        other_terms: prepared.other_terms,
        history: history
            .into_iter()
            .filter(|n| n.status != "DRAFT")
            .map(dto::notice)
            .collect(),
    }))
}

pub async fn save_notice_draft(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(case_id): Path<Uuid>,
    Json(input): Json<NoticeDraftInput>,
) -> Result<Json<Notice>, ApiFailure> {
    let row = notices::save_draft(
        &state.pool,
        &caller,
        case_id,
        DraftInput {
            subject: input.subject,
            body_text: input.body_text,
            proposed_period: input.proposed_period,
            other_terms: input.other_terms,
            recipient: input.recipient,
            cc: input.cc,
        },
    )
    .await?;
    Ok(Json(dto::notice(row)))
}

pub async fn notice_pdf(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(case_id): Path<Uuid>,
) -> Result<Json<Notice>, ApiFailure> {
    let (row, _) = notices::generate_pdf(&state.pool, &state.storage, &caller, case_id).await?;
    Ok(Json(dto::notice(row)))
}

pub async fn send_notice(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(case_id): Path<Uuid>,
    Json(req): Json<SendNoticeRequest>,
) -> Result<Json<Notice>, ApiFailure> {
    let row = notices::send(
        &state.pool,
        &state.storage,
        &caller,
        case_id,
        SendInput {
            to: req.to,
            cc: req.cc,
            email_body_text: req.email_body_text,
        },
    )
    .await?;
    Ok(Json(dto::notice(row)))
}
