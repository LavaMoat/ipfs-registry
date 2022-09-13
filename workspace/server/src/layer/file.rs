use std::path::PathBuf;

use async_trait::async_trait;
use axum::body::Bytes;

use ipfs_registry_core::{Artifact, ObjectKey};

use super::Layer;
use crate::{Error, Result};

pub struct FileLayer {
    directory: PathBuf,
}

impl FileLayer {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }
}

#[async_trait]
impl Layer for FileLayer {
    fn supports_content_id(&self) -> bool {
        false
    }

    async fn add_artifact(
        &self,
        data: Bytes,
        artifact: &Artifact,
    ) -> Result<ObjectKey> {
        let key = artifact.pointer_id();
        let path = self.directory.join(key.clone());
        if !path.exists() {
            tokio::fs::write(path, &data).await?;
        }
        Ok(ObjectKey::Pointer(key))
    }

    async fn get_artifact(&self, id: &ObjectKey) -> Result<Vec<u8>> {
        if let ObjectKey::Pointer(key) = id {
            let path = self.directory.join(key.clone());
            if path.exists() {
                Ok(tokio::fs::read(path).await?)
            } else {
                Err(Error::NotFile(path))
            }
        } else {
            Err(Error::BadObjectKey)
        }
    }
}
