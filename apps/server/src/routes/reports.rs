//! Phase 7 routes: reports (JSON / Excel / PDF) and the audit trail.

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::Json;
use renewal_api::*;
use renewal_db::audit::AuditFilter;
use renewal_services::reports::{self, ReportFilter, ReportKind, ReportTable};
use renewal_services::{audit, ServiceError};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::dto;
use crate::error::ApiFailure;
use crate::routes::master::list_query;
use crate::state::AppState;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportQuery {
    pub format: Option<String>,
    pub building_id: Option<String>,
    pub employee_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

pub enum ReportOut {
    Json(Json<ReportTable>),
    File(HeaderMap, Vec<u8>),
}

impl axum::response::IntoResponse for ReportOut {
    fn into_response(self) -> axum::response::Response {
        match self {
            ReportOut::Json(j) => j.into_response(),
            ReportOut::File(h, b) => (h, b).into_response(),
        }
    }
}

pub async fn report(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(kind): Path<String>,
    Query(q): Query<ReportQuery>,
) -> Result<ReportOut, ApiFailure> {
    let kind =
        ReportKind::parse(&kind).ok_or_else(|| ApiFailure(ServiceError::NotFound("report")))?;
    let f = ReportFilter {
        building_id: dto::uuid_opt(&q.building_id, "building")?,
        employee_id: dto::uuid_opt(&q.employee_id, "employee")?,
        from: dto::date_opt(&q.from, "from")?,
        to: dto::date_opt(&q.to, "to")?,
    };
    let table = reports::build(&state.pool, &caller, kind, &f).await?;
    let stamp = chrono::Utc::now().format("%Y-%m-%d").to_string();
    match q.format.as_deref().unwrap_or("json") {
        "xlsx" => {
            let bytes = reports::to_xlsx(&table)?;
            Ok(ReportOut::File(
                file_headers(
                    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                    &format!("{} {stamp}.xlsx", table.title),
                ),
                bytes,
            ))
        }
        "pdf" => {
            let bytes = reports::to_pdf(&table)?;
            Ok(ReportOut::File(
                file_headers("application/pdf", &format!("{} {stamp}.pdf", table.title)),
                bytes,
            ))
        }
        _ => Ok(ReportOut::Json(Json(table))),
    }
}

fn file_headers(content_type: &'static str, file_name: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    let safe: String = file_name
        .chars()
        .filter(|c| c.is_ascii() && *c != '"')
        .collect();
    h.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{safe}\""))
            .unwrap_or(HeaderValue::from_static("attachment")),
    );
    h
}

// ---------------------------------------------------------------- audit

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditQuery {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub actor_id: Option<String>,
    pub action: Option<String>,
}

pub async fn audit_list(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(p): Query<AuditQuery>,
) -> Result<Json<Page<AuditEntry>>, ApiFailure> {
    let lq = list_query(&ListParams {
        q: p.q,
        page: p.page,
        page_size: p.page_size,
        sort: None,
        dir: None,
    });
    let f = AuditFilter {
        entity_type: p.entity_type.filter(|s| !s.is_empty()),
        entity_id: dto::uuid_opt(&p.entity_id, "entity")?,
        actor_id: dto::uuid_opt(&p.actor_id, "actor")?,
        action: p.action.filter(|s| !s.is_empty()),
    };
    let res = audit::list(&state.pool, &caller, f, &lq).await?;
    Ok(Json(dto::page(
        res,
        lq.page,
        lq.page_size,
        dto::audit_entry,
    )))
}

pub async fn audit_for_entity(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path((entity_type, id)): Path<(String, Uuid)>,
) -> Result<Json<Vec<AuditEntry>>, ApiFailure> {
    let rows = audit::for_entity(&state.pool, &caller, &entity_type, id).await?;
    Ok(Json(rows.into_iter().map(dto::audit_entry).collect()))
}
