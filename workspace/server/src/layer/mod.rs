//! Traits and types for storage layers.
use async_trait::async_trait;
use axum::body::Bytes;
use web3_address::ethereum::Address;

use ipfs_registry_core::{
    Artifact, Pointer, Receipt, ObjectKey,
};

use serde_json::Value;

use crate::{Result, config::{ServerConfig, LayerConfig, RegistryConfig}};

pub(crate) mod ipfs;
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
        LayerConfig::Ipfs { url } => {
            Ok(Box::new(ipfs::IpfsLayer::new(url)?))
        }
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
    }
}

/// Build storage layers from the server configuration.
pub(crate) fn build(config: &ServerConfig) -> Result<Layers> {
    let mut storage = Vec::new();
    for layer in &config.storage.layers {
        storage.push(get_layer(layer, &config.registry)?);
    }

    Ok(Layers { storage })
}

/// Type for a collection of storage layer implementations.
pub(crate) struct Layers {
    storage: Vec<Box<dyn Layer + Send + Sync + 'static>>,
}

impl Layers {
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
    ) -> Result<String> {
        let has_mirrors = self.storage.len() > 1;
        if has_mirrors {
            let id = self.primary().add_blob(data.clone(), descriptor).await?;
            for mirror in self.storage.iter().skip(1) {
                mirror.add_blob(data.clone(), descriptor).await?;
            }
            Ok(id)
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
        object: ObjectKey,
        package: Value,
    ) -> Result<Receipt> {

        let has_mirrors = self.storage.len() > 1;
        if has_mirrors {
            let receipt = self
                .primary()
                .add_pointer(
                    signature.clone(),
                    address,
                    descriptor.clone(),
                    object.clone(),
                    package.clone(),
                )
                .await?;
            for mirror in self.storage.iter().skip(1) {
                mirror.add_pointer(
                    signature.clone(),
                    address,
                    descriptor.clone(),
                    object.clone(),
                    package.clone(),
                )
                .await?;
            }
            Ok(receipt)
        } else {
            self.primary()
                .add_pointer(
                    signature, address, descriptor, object, package,
                )
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

/// Trait for a storage layer.
#[async_trait]
pub trait Layer {
    /// Add a blob to the storage and return an identifier.
    async fn add_blob(
        &self,
        data: Bytes,
        descriptor: &Artifact,
    ) -> Result<String>;

    /// Get a blob from storage by identifier.
    async fn get_blob(&self, id: &ObjectKey) -> Result<Vec<u8>>;

    /// Add a pointer to the storage.
    async fn add_pointer(
        &self,
        signature: String,
        address: &Address,
        descriptor: Artifact,
        object: ObjectKey,
        package: Value,
    ) -> Result<Receipt>;

    /// Get a pointer from the storage.
    async fn get_pointer(
        &self,
        descriptor: &Artifact,
    ) -> Result<Option<Pointer>>;
}
