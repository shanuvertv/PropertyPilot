//! Default `StorageProvider`: bytes live in the `document_blobs` table (PLAN.md D10).

use async_trait::async_trait;
use renewal_db::{documents, PgPool};

use super::storage::{StorageError, StorageProvider, StoredObject};

#[derive(Clone)]
pub struct PgStorage {
    pool: PgPool,
}

impl PgStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StorageProvider for PgStorage {
    fn name(&self) -> &'static str {
        "postgres"
    }

    async fn put(&self, key: &str, content_type: &str, bytes: Vec<u8>) -> Result<(), StorageError> {
        documents::blob_put(&self.pool, key, content_type, &bytes)
            .await
            .map_err(|e| StorageError::Unavailable(e.to_string()))
    }

    async fn get(&self, key: &str) -> Result<StoredObject, StorageError> {
        match documents::blob_get(&self.pool, key).await {
            Ok(Some((content_type, bytes))) => Ok(StoredObject {
                key: key.to_owned(),
                content_type,
                bytes,
            }),
            Ok(None) => Err(StorageError::NotFound(key.to_owned())),
            Err(e) => Err(StorageError::Unavailable(e.to_string())),
        }
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        documents::blob_delete(&self.pool, key)
            .await
            .map_err(|e| StorageError::Unavailable(e.to_string()))
    }
}
