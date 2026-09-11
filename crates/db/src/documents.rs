use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgExecutor};
use uuid::Uuid;

use crate::DbResult;

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct DocumentRow {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_key: String,
    pub uploaded_by: Option<Uuid>,
    pub uploaded_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

const SELECT: &str = "SELECT d.id, d.entity_type, d.entity_id, d.file_name, d.content_type, d.size_bytes, d.storage_key,
       d.uploaded_by, u.name AS uploaded_by_name, d.created_at
  FROM documents d LEFT JOIN users u ON u.id = d.uploaded_by";

pub async fn for_entity<'e>(
    ex: impl PgExecutor<'e>,
    entity_type: &str,
    entity_id: Uuid,
) -> DbResult<Vec<DocumentRow>> {
    sqlx::query_as(&format!(
        "{SELECT} WHERE d.entity_type = $1 AND d.entity_id = $2 ORDER BY d.created_at DESC"
    ))
    .bind(entity_type)
    .bind(entity_id)
    .fetch_all(ex)
    .await
}

pub async fn find<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<Option<DocumentRow>> {
    sqlx::query_as(&format!("{SELECT} WHERE d.id = $1"))
        .bind(id)
        .fetch_optional(ex)
        .await
}

pub struct NewDocument<'a> {
    pub entity_type: &'a str,
    pub entity_id: Uuid,
    pub file_name: &'a str,
    pub content_type: &'a str,
    pub size_bytes: i64,
    pub storage_key: &'a str,
    pub uploaded_by: Uuid,
}

pub async fn insert<'e>(ex: impl PgExecutor<'e>, d: &NewDocument<'_>) -> DbResult<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO documents (entity_type, entity_id, file_name, content_type, size_bytes, storage_key, uploaded_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(d.entity_type)
    .bind(d.entity_id)
    .bind(d.file_name)
    .bind(d.content_type)
    .bind(d.size_bytes)
    .bind(d.storage_key)
    .bind(d.uploaded_by)
    .fetch_one(ex)
    .await
}

pub async fn delete<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> DbResult<()> {
    sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(id)
        .execute(ex)
        .await
        .map(|_| ())
}

// ---- blob storage backing the default StorageProvider

pub async fn blob_put<'e>(
    ex: impl PgExecutor<'e>,
    key: &str,
    content_type: &str,
    bytes: &[u8],
) -> DbResult<()> {
    sqlx::query(
        "INSERT INTO document_blobs (storage_key, content_type, bytes) VALUES ($1, $2, $3)
         ON CONFLICT (storage_key) DO UPDATE SET content_type = EXCLUDED.content_type, bytes = EXCLUDED.bytes",
    )
    .bind(key)
    .bind(content_type)
    .bind(bytes)
    .execute(ex)
    .await
    .map(|_| ())
}

pub async fn blob_get<'e>(
    ex: impl PgExecutor<'e>,
    key: &str,
) -> DbResult<Option<(String, Vec<u8>)>> {
    sqlx::query_as("SELECT content_type, bytes FROM document_blobs WHERE storage_key = $1")
        .bind(key)
        .fetch_optional(ex)
        .await
}

pub async fn blob_delete<'e>(ex: impl PgExecutor<'e>, key: &str) -> DbResult<()> {
    sqlx::query("DELETE FROM document_blobs WHERE storage_key = $1")
        .bind(key)
        .execute(ex)
        .await
        .map(|_| ())
}
