//! Traits and types for storage layers.
use async_trait::async_trait;
use axum::body::Bytes;
use web3_address::ethereum::Address;

use ipfs_registry_core::{Artifact, ObjectKey, Pointer};

use serde_json::Value;

use crate::{
    config::{LayerConfig, RegistryConfig, ServerConfig},
    Result,
};

pub(crate) mod ipfs;
pub(crate) mod memory;
pub(crate) mod s3;

pub(crate) const ROOT: &str = "ipkg-registry";
pub(crate) const NAME: &str = "pointer.json";
pub(crate) const BLOB: &str = "package.tgz";

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
        } => Ok(Box::new(s3::S3Layer::new(
            profile.to_string(),
            region.to_string(),
            bucket.to_string(),
            registry.mime.clone(),
        )?)),
        LayerConfig::Memory { .. } => {
            Ok(Box::new(memory::MemoryLayer::new()))
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
}

#[async_trait]
impl Layer for Layers {
    async fn add_blob(
        &self,
        data: Bytes,
        descriptor: &Artifact,
    ) -> Result<Vec<ObjectKey>> {
        let has_mirrors = self.storage.len() > 1;
        if has_mirrors {
            let mut keys = Vec::new();
            for layer in self.storage.iter() {
                let mut id = layer.add_blob(data.clone(), descriptor).await?;
                keys.append(&mut id);
            }
            Ok(keys)
        } else {
            self.primary().add_blob(data, descriptor).await
        }
    }

    async fn get_blob(&self, id: &ObjectKey) -> Result<Vec<u8>> {
        self.primary().get_blob(id).await
    }

    async fn add_pointer(
        &self,
        signature: String,
        address: &Address,
        descriptor: Artifact,
        objects: Vec<ObjectKey>,
        package: Value,
    ) -> Result<Vec<ObjectKey>> {
        let has_mirrors = self.storage.len() > 1;
        if has_mirrors {
            let mut keys = Vec::new();
            for layer in self.storage.iter() {
                let mut id = layer
                    .add_pointer(
                        signature.clone(),
                        address,
                        descriptor.clone(),
                        objects.clone(),
                        package.clone(),
                    )
                    .await?;

                keys.append(&mut id);
            }
            Ok(keys)
        } else {
            self.primary()
                .add_pointer(signature, address, descriptor, objects, package)
                .await
        }
    }

    async fn get_pointer(
        &self,
        descriptor: &Artifact,
    ) -> Result<Option<Pointer>> {
        self.primary().get_pointer(descriptor).await
    }
}

/// Get the key for a blob in non-content addressed storage layers.
pub(crate) fn get_blob_key(artifact: &Artifact) -> String {
    format!(
        "{}/{}/{}/{}/{}/{}",
        ROOT,
        &artifact.kind,
        &artifact.namespace,
        &artifact.package.name,
        &artifact.package.version,
        BLOB,
    )
}

/// Get the key for a pointer in non-content addressed storage layers.
pub(crate) fn get_pointer_key(artifact: &Artifact) -> String {
    format!(
        "{}/{}/{}/{}/{}/{}",
        ROOT,
        &artifact.kind,
        &artifact.namespace,
        &artifact.package.name,
        &artifact.package.version,
        NAME
    )
}

/// Trait for a storage layer.
#[async_trait]
pub trait Layer {
    /// Add a blob to the storage and return an identifier.
    async fn add_blob(
        &self,
        data: Bytes,
        artifact: &Artifact,
    ) -> Result<Vec<ObjectKey>>;

    /// Get a blob from storage by identifier.
    async fn get_blob(&self, id: &ObjectKey) -> Result<Vec<u8>>;

    /// Add a pointer to the storage.
    async fn add_pointer(
        &self,
        signature: String,
        address: &Address,
        artifact: Artifact,
        objects: Vec<ObjectKey>,
        package: Value,
    ) -> Result<Vec<ObjectKey>>;

    /// Get a pointer from the storage.
    async fn get_pointer(
        &self,
        artifact: &Artifact,
    ) -> Result<Option<Pointer>>;
}
