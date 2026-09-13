//! Attachments on buildings, tenants, contracts and notices.

use std::sync::Arc;

use renewal_core::Capability;
use renewal_db::documents::{self, DocumentRow, NewDocument};
use renewal_db::expenses;
use renewal_db::{buildings, contracts, tenants, PgPool};
use uuid::Uuid;

use crate::audit_log::{self, NONE};
use crate::error::{ServiceError, ServiceResult};
use crate::providers::StorageProvider;
use crate::session::Session;

pub const MAX_BYTES: usize = 25 * 1024 * 1024;

/// Which permission governs a document's parent entity.
fn caps(entity_type: &str) -> ServiceResult<(Capability, Capability)> {
    Ok(match entity_type {
        "building" => (Capability::ViewBuildings, Capability::ManageBuildings),
        "tenant" => (Capability::ViewTenants, Capability::ManageTenants),
        "contract" => (Capability::ViewContracts, Capability::ManageContracts),
        "notice" => (Capability::ViewRenewals, Capability::SendNotices),
        "expense" => (Capability::ViewExpenses, Capability::ManageExpenses),
        "cheque" => (Capability::ViewContracts, Capability::ManageContracts),
        _ => return Err(ServiceError::validation("unknown document entity type")),
    })
}

async fn parent_exists(pool: &PgPool, entity_type: &str, entity_id: Uuid) -> ServiceResult<bool> {
    Ok(match entity_type {
        "building" => buildings::find(pool, entity_id)
            .await?
            .is_some_and(|b| b.archived_at.is_none()),
        "tenant" => tenants::find(pool, entity_id)
            .await?
            .is_some_and(|t| t.archived_at.is_none()),
        "contract" => contracts::find(pool, entity_id).await?.is_some(),
        "notice" => true, // notices arrive in Phase 5
        "expense" => expenses::find(pool, entity_id).await?.is_some(),
        "cheque" => renewal_db::cheques::find(pool, entity_id).await?.is_some(),
        _ => false,
    })
}

pub async fn list(
    pool: &PgPool,
    caller: &Session,
    entity_type: &str,
    entity_id: Uuid,
) -> ServiceResult<Vec<DocumentRow>> {
    let (view, _) = caps(entity_type)?;
    caller.require(view)?;
    Ok(documents::for_entity(pool, entity_type, entity_id).await?)
}

pub struct Upload<'a> {
    pub entity_type: &'a str,
    pub entity_id: Uuid,
    pub file_name: &'a str,
    pub content_type: &'a str,
    pub bytes: Vec<u8>,
}

fn safe_file_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name).trim();
    let cleaned: String = base.chars().filter(|c| !c.is_control()).take(200).collect();
    if cleaned.is_empty() {
        "document".to_owned()
    } else {
        cleaned
    }
}

pub async fn upload(
    pool: &PgPool,
    storage: &Arc<dyn StorageProvider>,
    caller: &Session,
    up: Upload<'_>,
) -> ServiceResult<DocumentRow> {
    let (_, manage) = caps(up.entity_type)?;
    caller.require(manage)?;
    if up.bytes.is_empty() {
        return Err(ServiceError::validation("the file is empty"));
    }
    if up.bytes.len() > MAX_BYTES {
        return Err(ServiceError::validation("files are limited to 25 MB"));
    }
    if !parent_exists(pool, up.entity_type, up.entity_id).await? {
        return Err(ServiceError::NotFound("record to attach the document to"));
    }
    let file_name = safe_file_name(up.file_name);
    let content_type = if up.content_type.trim().is_empty() {
        "application/octet-stream"
    } else {
        up.content_type.trim()
    };
    let key = format!("documents/{}/{}", up.entity_type, Uuid::new_v4());
    let size = up.bytes.len() as i64;
    storage
        .put(&key, content_type, up.bytes)
        .await
        .map_err(|e| ServiceError::Internal(format!("storing the file failed: {e}")))?;

    let mut tx = pool.begin().await?;
    let id = documents::insert(
        &mut *tx,
        &NewDocument {
            entity_type: up.entity_type,
            entity_id: up.entity_id,
            file_name: &file_name,
            content_type,
            size_bytes: size,
            storage_key: &key,
            uploaded_by: caller.user_id,
        },
    )
    .await?;
    let row = documents::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("document"))?;
    audit_log::log(
        &mut *tx,
        caller,
        up.entity_type,
        up.entity_id,
        "DOCUMENT_ADDED",
        NONE,
        Some(&row),
    )
    .await?;
    tx.commit().await?;
    Ok(row)
}

pub struct Download {
    pub meta: DocumentRow,
    pub bytes: Vec<u8>,
}

pub async fn download(
    pool: &PgPool,
    storage: &Arc<dyn StorageProvider>,
    caller: &Session,
    id: Uuid,
) -> ServiceResult<Download> {
    let meta = documents::find(pool, id)
        .await?
        .ok_or(ServiceError::NotFound("document"))?;
    let (view, _) = caps(&meta.entity_type)?;
    caller.require(view)?;
    let obj = storage
        .get(&meta.storage_key)
        .await
        .map_err(|e| ServiceError::Internal(format!("reading the file failed: {e}")))?;
    Ok(Download {
        meta,
        bytes: obj.bytes,
    })
}

pub async fn delete(
    pool: &PgPool,
    storage: &Arc<dyn StorageProvider>,
    caller: &Session,
    id: Uuid,
) -> ServiceResult<()> {
    let meta = documents::find(pool, id)
        .await?
        .ok_or(ServiceError::NotFound("document"))?;
    let (_, manage) = caps(&meta.entity_type)?;
    caller.require(manage)?;
    let mut tx = pool.begin().await?;
    documents::delete(&mut *tx, id).await?;
    audit_log::log(
        &mut *tx,
        caller,
        &meta.entity_type,
        meta.entity_id,
        "DOCUMENT_REMOVED",
        Some(&meta),
        NONE,
    )
    .await?;
    tx.commit().await?;
    let _ = storage.delete(&meta.storage_key).await;
    Ok(())
}
