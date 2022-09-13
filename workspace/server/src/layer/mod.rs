//! Traits and types for storage layers.
use async_trait::async_trait;
use axum::body::Bytes;
use cid::Cid;

use ipfs_registry_core::{Artifact, ObjectKey};

use crate::{
    config::{LayerConfig, RegistryConfig, ServerConfig},
    Error, Result,
};

pub(crate) mod file;
pub(crate) mod ipfs;
pub(crate) mod memory;
pub(crate) mod s3;

/// Convert a configuration into a layer implementation.
fn get_layer(
    config: &LayerConfig,
    registry: &RegistryConfig,
) -> Result<Box<dyn Layer + Send + Sync + 'static>> {
    match config {
        LayerConfig::Ipfs { url } => Ok(Box::new(ipfs::IpfsLayer::new(url)?)),
        LayerConfig::Aws {
            profile,
            region,
            bucket,
            prefix,
        } => Ok(Box::new(s3::S3Layer::new(
            profile.to_string(),
            region.to_string(),
            bucket.to_string(),
            registry.mime.clone(),
            prefix.clone(),
        )?)),
        LayerConfig::Memory { .. } => {
            Ok(Box::new(memory::MemoryLayer::new()))
        }
        LayerConfig::File { directory } => {
            Ok(Box::new(file::FileLayer::new(directory.clone())))
        }
    }
}

/// Build storage layers from the server configuration.
pub fn build(config: &ServerConfig) -> Result<Layers> {
    let mut storage = Vec::new();
    for layer in &config.storage.layers {
        storage.push(get_layer(layer, &config.registry)?);
    }

    Ok(Layers { storage })
}

/// Type for a collection of storage layer implementations.
pub struct Layers {
    storage: Vec<Box<dyn Layer + Send + Sync + 'static>>,
}

impl Layers {
    /// Publish an artifact to all storage layers.
    pub async fn publish(
        &self,
        data: Bytes,
        artifact: &Artifact,
    ) -> Result<Vec<ObjectKey>> {
        // Do it like this to avoid an unnecessary clone() on the
        // buffer when only a single storage layer is configured
        let has_mirrors = self.storage.len() > 1;
        if has_mirrors {
            let mut keys = Vec::new();
            for layer in self.storage.iter() {
                let id = layer.add_artifact(data.clone(), artifact).await?;
                keys.push(id);
            }
            Ok(keys)
        } else {
            let primary = self
                .storage
                .get(0)
                .expect("failed to get primary storage layer");
            Ok(vec![primary.add_artifact(data, artifact).await?])
        }
    }

    /// Fetch an artifact from the storage layers.
    pub async fn fetch(
        &self,
        pointer_id: &str,
        content_id: Option<&Cid>,
    ) -> Result<Vec<u8>> {
        let pointer_id = ObjectKey::Pointer(pointer_id.to_string());
        let content_id = content_id.map(|c| ObjectKey::Cid(c.clone()));

        let len = self.storage.len();
        for (index, layer) in self.storage.iter().enumerate() {
            let is_last = index == len - 1;
            let result = if layer.supports_content_id() {
                if let Some(content_id) = &content_id {
                    layer.get_artifact(content_id).await
                } else {
                    continue;
                }
            } else {
                layer.get_artifact(&pointer_id).await
            };

            match result {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::error!("{}", e);
                    if is_last {
                        return Err(e);
                    }
                }
            }
        }

        Err(Error::ArtifactNotFound(
            pointer_id.to_string(),
            content_id.map(|c| c.to_string()),
        ))
    }
}

/// Trait for a storage layer.
#[async_trait]
pub trait Layer {
    /// Determine if this layer supports a content identifier.
    fn supports_content_id(&self) -> bool;

    /// Add an artifact to the storage layer and return an identifier.
    async fn add_artifact(
        &self,
        data: Bytes,
        artifact: &Artifact,
    ) -> Result<ObjectKey>;

    /// Get an artifact from storage by identifier.
    async fn get_artifact(&self, id: &ObjectKey) -> Result<Vec<u8>>;
}
