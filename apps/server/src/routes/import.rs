//! Phase 8: Excel import (preview, then commit).

use axum::extract::{Multipart, State};
use axum::Json;
use renewal_services::import::{self, ImportPreview, ImportResult};
use renewal_services::ServiceError;

use crate::auth::CurrentUser;
use crate::error::ApiFailure;
use crate::state::{AppState, LiveEvent};

async fn read_file(mut multipart: Multipart) -> Result<Vec<u8>, ApiFailure> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiFailure(ServiceError::Validation(format!("bad upload: {e}"))))?
    {
        if field.name() == Some("file") {
            let bytes = field
                .bytes()
                .await
                .map_err(|e| ApiFailure(ServiceError::Validation(format!("bad upload: {e}"))))?;
            return Ok(bytes.to_vec());
        }
    }
    Err(ApiFailure(ServiceError::Validation(
        "attach the workbook in the `file` field".into(),
    )))
}

pub async fn preview(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    multipart: Multipart,
) -> Result<Json<ImportPreview>, ApiFailure> {
    let bytes = read_file(multipart).await?;
    Ok(Json(import::preview(&state.pool, &caller, &bytes).await?))
}

pub async fn commit(
    State(state): State<AppState>,
    CurrentUser(caller): CurrentUser,
    multipart: Multipart,
) -> Result<Json<ImportResult>, ApiFailure> {
    let bytes = read_file(multipart).await?;
    let result = import::commit(&state.pool, &caller, &bytes).await?;
    state.publish(LiveEvent::data());
    Ok(Json(result))
}
