//! Phase 6 routes: notifications, live events (SSE), reminder schedule, org settings, sweep.

use std::convert::Infallible;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use futures_util::stream::Stream;
use renewal_api::*;
use renewal_services::mail_settings::{self, MailConfigInput, MailConfigView};
use renewal_services::pdf::Letterhead;
use renewal_services::settings::{self, OrgSettings, RuleDraft};
use renewal_services::{auth, notifications, sweep};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::dto;
use crate::error::ApiFailure;
use crate::state::{AppState, LiveEvent};

// ---------------------------------------------------------------- notifications

#[derive(serde::Deserialize)]
pub struct NotifQuery {
    pub unread: Option<bool>,
}

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(q): Query<NotifQuery>,
) -> Result<Json<Vec<Notification>>, ApiFailure> {
    let rows = notifications::list(&state.pool, &caller, q.unread.unwrap_or(false)).await?;
    Ok(Json(rows.into_iter().map(dto::notification).collect()))
}

pub async fn unread_count(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<UnreadCount>, ApiFailure> {
    Ok(Json(UnreadCount {
        unread: notifications::unread_count(&state.pool, &caller).await?,
    }))
}

pub async fn mark_read(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiFailure> {
    notifications::mark_read(&state.pool, &caller, id).await?;
    state.publish(LiveEvent::notifications(Some(caller.user_id)));
    Ok(StatusCode::NO_CONTENT)
}

pub async fn mark_all_read(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<StatusCode, ApiFailure> {
    notifications::mark_all_read(&state.pool, &caller).await?;
    state.publish(LiveEvent::notifications(Some(caller.user_id)));
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------- live events (SSE)

#[derive(serde::Deserialize)]
pub struct EventsQuery {
    pub token: String,
}

/// `GET /api/events?token=…` — EventSource cannot set headers, so the bearer token
/// travels as a query parameter (HTTPS in production). Emits `notifications` and
/// `data` events; the client invalidates its caches on each.
pub async fn events(
    State(state): State<AppState>,
    Query(q): Query<EventsQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiFailure> {
    let session = auth::authenticate(&state.pool, &q.token)
        .await?
        .ok_or_else(ApiFailure::unauthorized)?;
    let user_id = session.user_id;
    let rx = state.events.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(move |ev| match ev {
        Ok(ev) if ev.user_id.is_none() || ev.user_id == Some(user_id) => {
            Some(Ok(Event::default().event(ev.kind).data(ev.kind)))
        }
        _ => None,
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

// ---------------------------------------------------------------- settings

fn org_dto(o: OrgSettings) -> OrgSettingsDto {
    OrgSettingsDto {
        org_name: o.org_name,
        timezone: o.timezone,
        thresholds: o.thresholds,
        auto_open_case: o.auto_open_case,
        completed_window_days: o.completed_window_days,
        letterhead: LetterheadDto {
            company_name: o.letterhead.company_name,
            address_lines: o.letterhead.address_lines,
            phone: o.letterhead.phone,
            email: o.letterhead.email,
            footer: o.letterhead.footer,
        },
    }
}

pub async fn get_org(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<OrgSettingsDto>, ApiFailure> {
    caller.require(renewal_core::Capability::ViewDashboard)?;
    Ok(Json(org_dto(settings::org(&state.pool).await?)))
}

pub async fn save_org(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Json(input): Json<OrgSettingsDto>,
) -> Result<Json<OrgSettingsDto>, ApiFailure> {
    let saved = settings::save_org(
        &state.pool,
        &caller,
        OrgSettings {
            org_name: input.org_name,
            timezone: input.timezone,
            thresholds: input.thresholds,
            auto_open_case: input.auto_open_case,
            completed_window_days: input.completed_window_days,
            letterhead: Letterhead {
                company_name: input.letterhead.company_name,
                address_lines: input
                    .letterhead
                    .address_lines
                    .into_iter()
                    .map(|l| l.trim().to_owned())
                    .filter(|l| !l.is_empty())
                    .collect(),
                phone: input.letterhead.phone,
                email: input.letterhead.email,
                footer: input.letterhead.footer,
            },
        },
    )
    .await?;
    state.publish(LiveEvent::data());
    Ok(Json(org_dto(saved)))
}

pub async fn get_rules(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<Vec<ReminderRuleDto>>, ApiFailure> {
    Ok(Json(
        settings::reminder_rules(&state.pool, &caller)
            .await?
            .into_iter()
            .map(dto::reminder_rule)
            .collect(),
    ))
}

pub async fn save_rules(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Json(items): Json<Vec<ReminderRuleDto>>,
) -> Result<Json<Vec<ReminderRuleDto>>, ApiFailure> {
    let drafts = items
        .into_iter()
        .map(|r| {
            Ok(RuleDraft {
                id: dto::uuid_opt(&r.id, "rule")?,
                days_before: r.days_before,
                label: r.label,
                notify_in_app: r.notify_in_app,
                email_assigned_employee: r.email_assigned_employee,
                mark_urgent: r.mark_urgent,
                active: r.active,
            })
        })
        .collect::<Result<Vec<_>, ApiFailure>>()?;
    let rows = settings::save_reminder_rules(&state.pool, &caller, drafts).await?;
    Ok(Json(rows.into_iter().map(dto::reminder_rule).collect()))
}

/// Admin "Run sweep now" (spec §8; PLAN.md §6.5).
pub async fn run_sweep(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<SweepSummaryDto>, ApiFailure> {
    caller.require(renewal_core::Capability::ManageSettings)?;
    let s = sweep::run(&state.pool, Some(&caller)).await?;
    state.publish(LiveEvent::notifications(None));
    state.publish(LiveEvent::data());
    Ok(Json(dto::sweep_summary(&s)))
}

// ---------------------------------------------------------------- mail settings (Settings → Email sending)

pub async fn mail_settings_get(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<MailConfigView>, ApiFailure> {
    Ok(Json(mail_settings::view(&state.pool, &caller).await?))
}

pub async fn mail_settings_put(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Json(input): Json<MailConfigInput>,
) -> Result<Json<MailConfigView>, ApiFailure> {
    Ok(Json(
        mail_settings::save(&state.pool, &caller, input).await?,
    ))
}

pub async fn mail_settings_test(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Json(req): Json<MailTestRequest>,
) -> Result<Json<MailTestResult>, ApiFailure> {
    let org = settings::org(&state.pool).await?;
    let receipt = mail_settings::send_test(&caller, &state.mail, &req.to, &org.org_name).await?;
    let d = state.mail_dynamic.describe().await;
    Ok(Json(MailTestResult {
        provider: d.provider,
        provider_message_id: receipt.provider_message_id,
    }))
}
