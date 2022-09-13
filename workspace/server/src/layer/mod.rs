//! Traits and types for storage layers.
use async_trait::async_trait;
use axum::body::Bytes;

use ipfs_registry_core::{Artifact, ObjectKey};

use crate::{
    config::{LayerConfig, RegistryConfig, ServerConfig},
    Result,
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
    /// Primary storage layer.
    ///
    /// The configuration loader ensures we always have at least one
    /// layer configuration so we can be certain we have a primary layer.
    #[allow(clippy::borrowed_box)]
    fn primary(&self) -> &Box<dyn Layer + Send + Sync + 'static> {
        self.storage.get(0).unwrap()
    }

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
            Ok(vec![self.primary().add_artifact(data, artifact).await?])
        }
    }

    /// Fetch an artifact from the storage layers.
    pub async fn fetch(&self, id: &ObjectKey) -> Result<Vec<u8>> {
        // FIXME: find first responding layer that returns an object...
        self.primary().get_artifact(id).await
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
