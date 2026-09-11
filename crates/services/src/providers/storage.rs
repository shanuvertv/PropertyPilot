use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct StoredObject {
    pub key: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("object {0} not found")]
    NotFound(String),
    #[error("storage unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait StorageProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn put(&self, key: &str, content_type: &str, bytes: Vec<u8>) -> Result<(), StorageError>;
    async fn get(&self, key: &str) -> Result<StoredObject, StorageError>;
    async fn delete(&self, key: &str) -> Result<(), StorageError>;
}

/// In-memory store for tests and local development.
#[derive(Debug, Default)]
pub struct MemoryStorage {
    objects: RwLock<HashMap<String, StoredObject>>,
}

#[async_trait]
impl StorageProvider for MemoryStorage {
    fn name(&self) -> &'static str {
        "memory"
    }

    async fn put(&self, key: &str, content_type: &str, bytes: Vec<u8>) -> Result<(), StorageError> {
        self.objects.write().await.insert(
            key.to_owned(),
            StoredObject {
                key: key.to_owned(),
                content_type: content_type.to_owned(),
                bytes,
            },
        );
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<StoredObject, StorageError> {
        self.objects
            .read()
            .await
            .get(key)
            .cloned()
            .ok_or_else(|| StorageError::NotFound(key.to_owned()))
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.objects.write().await.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_storage_round_trip() {
        let s = MemoryStorage::default();
        s.put("contracts/c1.pdf", "application/pdf", b"%PDF".to_vec())
            .await
            .unwrap();
        let got = s.get("contracts/c1.pdf").await.unwrap();
        assert_eq!(got.content_type, "application/pdf");
        assert_eq!(got.bytes, b"%PDF");
        s.delete("contracts/c1.pdf").await.unwrap();
        assert!(matches!(
            s.get("contracts/c1.pdf").await,
            Err(StorageError::NotFound(_))
        ));
    }
}
