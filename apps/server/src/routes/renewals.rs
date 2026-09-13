//! Phase 4 routes: renewal cases, tenant responses, checklist, completion, follow-ups.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use renewal_api::*;
use renewal_db::follow_ups::{FollowUpFilter, Scope};
use renewal_db::renewals::CaseFilter;
use renewal_services::follow_ups::FollowUpInput as SvcFollowUpInput;
use renewal_services::renewals::{
    self, allowed_transitions, CompletionInput, ResponseInput, TemplateInput,
};
use renewal_services::{contracts, follow_ups};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::dto;
use crate::error::ApiFailure;
use crate::routes::master::list_query;
use crate::state::AppState;

// ---------------------------------------------------------------- cases

pub async fn list_cases(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(p): Query<CaseListParams>,
) -> Result<Json<Page<RenewalCase>>, ApiFailure> {
    let q = list_query(&p.list());
    let f = CaseFilter {
        status: p.status.map(|s| vec![s.to_string()]),
        notice_status: p.notice_status.map(|s| vec![s.to_string()]),
        band: p.band.map(|b| b.to_string()),
        open_only: p.open_only.unwrap_or(false),
        assigned_employee_id: dto::uuid_opt(&p.assigned_employee_id, "assigned employee")?,
        building_id: dto::uuid_opt(&p.building_id, "building")?,
        latest_response: p.latest_response.map(|r| vec![r.to_string()]),
        ..Default::default()
    };
    let res = renewals::list(&state.pool, &caller, &f, &q).await?;
    Ok(Json(dto::page(res, q.page, q.page_size, dto::case)))
}

pub async fn get_case(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RenewalCaseDetail>, ApiFailure> {
    let case = renewals::get(&state.pool, &caller, id).await?;
    let contract = contracts::get(&state.pool, &caller, case.contract_id).await?;
    let responses = renewals::responses(&state.pool, &caller, id).await?;
    let fus = if caller.role.allows(renewal_core::Capability::ViewFollowUps) {
        follow_ups::for_case(&state.pool, &caller, id).await?
    } else {
        vec![]
    };
    let checklist = renewals::checklist(&state.pool, &caller, id).await?;
    let status: renewal_core::RenewalStatus = case
        .status
        .parse()
        .unwrap_or(renewal_core::RenewalStatus::NotStarted);
    Ok(Json(RenewalCaseDetail {
        case: dto::case(case),
        contract: dto::contract(contract),
        responses: responses.into_iter().map(dto::response).collect(),
        follow_ups: fus.into_iter().map(dto::follow_up).collect(),
        checklist: checklist.into_iter().map(dto::checklist_item).collect(),
        allowed_transitions: allowed_transitions(status),
    }))
}

pub async fn set_status(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SetRenewalStatusRequest>,
) -> Result<Json<RenewalCase>, ApiFailure> {
    Ok(Json(dto::case(
        renewals::set_status(&state.pool, &caller, id, req.status).await?,
    )))
}

pub async fn assign(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AssignRequest>,
) -> Result<Json<RenewalCase>, ApiFailure> {
    let emp = dto::uuid_opt(&req.assigned_employee_id, "assigned employee")?;
    Ok(Json(dto::case(
        renewals::assign(&state.pool, &caller, id, emp).await?,
    )))
}

pub async fn set_notes(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SetNotesRequest>,
) -> Result<Json<RenewalCase>, ApiFailure> {
    Ok(Json(dto::case(
        renewals::set_notes(&state.pool, &caller, id, req.notes).await?,
    )))
}

pub async fn record_response(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RecordResponseRequest>,
) -> Result<Json<RenewalCase>, ApiFailure> {
    let input = ResponseInput {
        response: req.response,
        response_date: dto::date(&req.response_date, "response date")?,
        notes: req.notes,
        follow_up_date: dto::date_opt(&req.follow_up_date, "follow-up date")?,
        follow_up_type: req.follow_up_type.map(|t| t.to_string()),
    };
    Ok(Json(dto::case(
        renewals::record_response(&state.pool, &caller, id, input).await?,
    )))
}

pub async fn set_checklist_item(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path((id, item_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<SetChecklistItemRequest>,
) -> Result<Json<Vec<ChecklistItem>>, ApiFailure> {
    let items = renewals::set_checklist_item(&state.pool, &caller, id, item_id, req.done).await?;
    Ok(Json(items.into_iter().map(dto::checklist_item).collect()))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResult {
    pub case: RenewalCase,
    pub new_contract: Contract,
}

pub async fn complete(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CompleteRenewalRequest>,
) -> Result<Json<CompletionResult>, ApiFailure> {
    let input = CompletionInput {
        contract_number: req.contract_number,
        start_date: dto::date(&req.start_date, "new start date")?,
        end_date: dto::date(&req.end_date, "new end date")?,
        rent_terms: req.rent_terms,
        rent_amount_minor: req
            .rent_amount
            .map(|a| dto::minor(a, "rent amount"))
            .transpose()?,
        notes: req.notes,
    };
    let (case, new_contract) = renewals::complete(&state.pool, &caller, id, input).await?;
    Ok(Json(CompletionResult {
        case: dto::case(case),
        new_contract: dto::contract(new_contract),
    }))
}

// ---------------------------------------------------------------- checklist template (Admin)

pub async fn get_template(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
) -> Result<Json<Vec<ChecklistTemplateItem>>, ApiFailure> {
    Ok(Json(
        renewals::template(&state.pool, &caller)
            .await?
            .into_iter()
            .map(dto::template_item)
            .collect(),
    ))
}

pub async fn save_template(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Json(items): Json<Vec<ChecklistTemplateItem>>,
) -> Result<Json<Vec<ChecklistTemplateItem>>, ApiFailure> {
    let inputs = items
        .into_iter()
        .map(|i| {
            Ok(TemplateInput {
                id: dto::uuid_opt(&i.id, "checklist item")?,
                label: i.label,
                required: i.required,
                active: i.active,
            })
        })
        .collect::<Result<Vec<_>, ApiFailure>>()?;
    let rows = renewals::save_template(&state.pool, &caller, inputs).await?;
    Ok(Json(rows.into_iter().map(dto::template_item).collect()))
}

// ---------------------------------------------------------------- follow-ups

pub async fn list_follow_ups(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(p): Query<FollowUpListParams>,
) -> Result<Json<Page<FollowUp>>, ApiFailure> {
    let mut q = list_query(&p.list());
    if q.sort.is_none() {
        q.sort = Some("due_date".into());
    }
    let f = FollowUpFilter {
        scope: Some(Scope::parse(p.scope.as_deref())),
        case_id: None,
        assigned_employee_id: if p.mine.unwrap_or(false) {
            Some(caller.user_id)
        } else {
            dto::uuid_opt(&p.assigned_employee_id, "assigned employee")?
        },
        follow_up_type: p.follow_up_type.map(|t| t.to_string()),
        building_id: dto::uuid_opt(&p.building_id, "building")?,
        due_from: dto::date_opt(&p.from, "from")?,
        due_to: dto::date_opt(&p.to, "to")?,
    };
    let res = follow_ups::list(&state.pool, &caller, &f, &q).await?;
    Ok(Json(dto::page(res, q.page, q.page_size, dto::follow_up)))
}

pub async fn follow_up_counts(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(p): Query<FollowUpListParams>,
) -> Result<Json<FollowUpCounts>, ApiFailure> {
    let c = follow_ups::counts(&state.pool, &caller, p.mine.unwrap_or(false)).await?;
    Ok(Json(dto::follow_up_counts(c)))
}

fn follow_up_input(i: FollowUpInput) -> Result<SvcFollowUpInput, ApiFailure> {
    Ok(SvcFollowUpInput {
        due_date: dto::date(&i.due_date, "due date")?,
        follow_up_type: i.follow_up_type,
        assigned_employee_id: dto::uuid_opt(&i.assigned_employee_id, "assigned employee")?,
        notes: i.notes,
    })
}

pub async fn create_follow_up(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(case_id): Path<Uuid>,
    Json(input): Json<FollowUpInput>,
) -> Result<(StatusCode, Json<FollowUp>), ApiFailure> {
    let row = follow_ups::create(&state.pool, &caller, case_id, follow_up_input(input)?).await?;
    Ok((StatusCode::CREATED, Json(dto::follow_up(row))))
}

pub async fn update_follow_up(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<FollowUpInput>,
) -> Result<Json<FollowUp>, ApiFailure> {
    let row = follow_ups::update(&state.pool, &caller, id, follow_up_input(input)?).await?;
    Ok(Json(dto::follow_up(row)))
}

pub async fn set_follow_up_status(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SetFollowUpStatusRequest>,
) -> Result<Json<FollowUp>, ApiFailure> {
    let row = follow_ups::set_status(&state.pool, &caller, id, req.status).await?;
    Ok(Json(dto::follow_up(row)))
}
