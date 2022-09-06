use std::collections::HashMap;

use async_trait::async_trait;
use axum::body::Bytes;

use tokio::sync::RwLock;


use ipfs_registry_core::{
    Artifact, ObjectKey, Pointer,
};

use super::{get_blob_key, get_pointer_key, Layer};
use crate::{Error, Result};

pub struct MemoryLayer {
    files: RwLock<HashMap<String, Vec<u8>>>,
}

impl MemoryLayer {
    pub fn new() -> Self {
        Self {
            files: RwLock::new(Default::default()),
        }
    }
}

#[async_trait]
impl Layer for MemoryLayer {
    async fn add_blob(
        &self,
        data: Bytes,
        artifact: &Artifact,
    ) -> Result<Vec<ObjectKey>> {
        let key = get_blob_key(artifact);
        let mut writer = self.files.write().await;
        writer.insert(key.clone(), data.to_vec());
        Ok(vec![ObjectKey::Key(key)])
    }

    async fn get_blob(&self, id: &ObjectKey) -> Result<Vec<u8>> {
        if let ObjectKey::Key(key) = id {
            let reader = self.files.read().await;
            let result = reader
                .get(key)
                .ok_or_else(|| Error::ObjectMissing(key.to_string()))?;
            Ok(result.clone())
        } else {
            Err(Error::BadObjectKey)
        }
    }

    async fn add_pointer(&self, doc: Pointer) -> Result<Vec<ObjectKey>> {
        let key = get_pointer_key(&doc.definition.artifact);
        let data = serde_json::to_vec_pretty(&doc)?;
        let mut writer = self.files.write().await;
        writer.insert(key.clone(), data);
        Ok(vec![ObjectKey::Key(key)])
    }

    async fn get_pointer(
        &self,
        artifact: &Artifact,
    ) -> Result<Option<Pointer>> {
        let key = get_pointer_key(artifact);
        let reader = self.files.read().await;
        let result = if let Some(res) = reader.get(&key) {
            let doc: Pointer = serde_json::from_slice(res)?;
            Some(doc)
        } else {
            None
        };
        Ok(result)
    }
}
