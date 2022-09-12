use std::collections::HashMap;

use async_trait::async_trait;
use axum::body::Bytes;

use tokio::sync::RwLock;

use ipfs_registry_core::{Artifact, ObjectKey};

use super::Layer;
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
        let key = artifact.key();
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
}
