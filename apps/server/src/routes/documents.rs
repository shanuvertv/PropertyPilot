use axum::body::Bytes;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::Json;
use renewal_api::DocumentInfo;
use renewal_services::documents::{self, Upload};
use renewal_services::ServiceError;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::dto;
use crate::error::ApiFailure;
use crate::state::AppState;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocQuery {
    pub entity_type: String,
    pub entity_id: String,
}

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Query(q): Query<DocQuery>,
) -> Result<Json<Vec<DocumentInfo>>, ApiFailure> {
    let entity_id = dto::uuid(&q.entity_id, "entity")?;
    let rows = documents::list(&state.pool, &caller, &q.entity_type, entity_id).await?;
    Ok(Json(rows.into_iter().map(dto::document).collect()))
}

/// `multipart/form-data` with fields `entityType`, `entityId`, `file`.
pub async fn upload(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<DocumentInfo>), ApiFailure> {
    let mut entity_type: Option<String> = None;
    let mut entity_id: Option<String> = None;
    let mut file: Option<(String, String, Bytes)> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiFailure(ServiceError::Validation(format!("bad upload: {e}"))))?
    {
        match field.name().unwrap_or("") {
            "entityType" => entity_type = Some(field.text().await.map_err(bad)?),
            "entityId" => entity_id = Some(field.text().await.map_err(bad)?),
            "file" => {
                let name = field.file_name().unwrap_or("document").to_owned();
                let ct = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_owned();
                let bytes = field.bytes().await.map_err(bad)?;
                file = Some((name, ct, bytes));
            }
            _ => {}
        }
    }
    let entity_type = entity_type
        .ok_or_else(|| ApiFailure(ServiceError::Validation("entityType is required".into())))?;
    let entity_id = dto::uuid(&entity_id.unwrap_or_default(), "entityId")?;
    let (file_name, content_type, bytes) = file.ok_or_else(|| {
        ApiFailure(ServiceError::Validation(
            "attach a file in the `file` field".into(),
        ))
    })?;
    let row = documents::upload(
        &state.pool,
        &state.storage,
        &caller,
        Upload {
            entity_type: &entity_type,
            entity_id,
            file_name: &file_name,
            content_type: &content_type,
            bytes: bytes.to_vec(),
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(dto::document(row))))
}

fn bad(e: axum::extract::multipart::MultipartError) -> ApiFailure {
    ApiFailure(ServiceError::Validation(format!("bad upload: {e}")))
}

pub async fn download(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<(HeaderMap, Vec<u8>), ApiFailure> {
    let dl = documents::download(&state.pool, &state.storage, &caller, id).await?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&dl.meta.content_type)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    let safe: String = dl
        .meta
        .file_name
        .chars()
        .filter(|c| c.is_ascii() && *c != '"' && !c.is_control())
        .collect();
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{safe}\""))
            .unwrap_or(HeaderValue::from_static("attachment")),
    );
    Ok((headers, dl.bytes))
}

pub async fn delete(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiFailure> {
    documents::delete(&state.pool, &state.storage, &caller, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
