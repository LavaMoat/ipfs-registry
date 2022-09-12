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
    async fn add_blob(
        &self,
        data: Bytes,
        artifact: &Artifact,
    ) -> Result<Vec<ObjectKey>> {
        let key = artifact.key();
        let path = self.directory.join(key.clone());
        if !path.exists() {
            tokio::fs::write(path, &data).await?;
        }
        Ok(vec![ObjectKey::Key(key)])
    }

    async fn get_blob(&self, id: &ObjectKey) -> Result<Vec<u8>> {
        if let ObjectKey::Key(key) = id {
            let path = self.directory.join(key.clone());
            if path.exists() {
                let contents = tokio::fs::read(path).await?;
                Ok(contents)
            } else {
                Err(Error::NotFile(path))
            }
        } else {
            Err(Error::BadObjectKey)
        }
    }
}
