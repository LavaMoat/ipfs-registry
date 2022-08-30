//! Traits and types for storage layers.
use async_trait::async_trait;
use axum::body::Bytes;
use semver::Version;
use web3_address::ethereum::Address;

use ipfs_registry_core::{Descriptor, PackagePointer, Receipt, RegistryKind};

use serde_json::Value;

pub(crate) mod ipfs;

use crate::Result;

/// Type for a primary and backup layer.
pub(crate) struct Layers {
    pub primary: Box<dyn Layer + Send + Sync + 'static>,
    pub backup: Option<Box<dyn Layer + Send + Sync + 'static>>,
}

#[async_trait]
impl Layer for Layers {
    async fn add_blob(&self, data: Bytes) -> Result<String> {
        if let Some(backup) = &self.backup {
            let id = self.primary.add_blob(data.clone()).await?;
            backup.add_blob(data).await?;
            Ok(id)
        } else {
            self.primary.add_blob(data).await
        }
    }

    async fn get_blob(&self, id: &str) -> Result<Vec<u8>> {
        self.primary.get_blob(id).await
    }

    async fn add_pointer(
        &self,
        kind: RegistryKind,
        signature: String,
        address: &Address,
        descriptor: Descriptor,
        archive_id: String,
        package: Value,
    ) -> Result<Receipt> {
        if let Some(backup) = &self.backup {
            let receipt = self
                .primary
                .add_pointer(
                    kind,
                    signature.clone(),
                    address,
                    descriptor.clone(),
                    archive_id.clone(),
                    package.clone(),
                )
                .await?;
            backup
                .add_pointer(
                    kind, signature, address, descriptor, archive_id, package,
                )
                .await?;
            Ok(receipt)
        } else {
            self.primary
                .add_pointer(
                    kind, signature, address, descriptor, archive_id, package,
                )
                .await
        }
    }

    async fn get_pointer(
        &self,
        kind: RegistryKind,
        address: &Address,
        name: &str,
        version: &Version,
    ) -> Result<Option<PackagePointer>> {
        self.primary.get_pointer(kind, address, name, version).await
    }
}

/// Trait for a storage layer.
#[async_trait]
pub trait Layer {
    /// Add a blob to the storage and return an identifier.
    async fn add_blob(&self, data: Bytes) -> Result<String>;

    /// Get a blob from storage by identifier.
    async fn get_blob(&self, id: &str) -> Result<Vec<u8>>;

    /// Add a pointer to the storage.
    async fn add_pointer(
        &self,
        kind: RegistryKind,
        signature: String,
        address: &Address,
        descriptor: Descriptor,
        archive_id: String,
        package: Value,
    ) -> Result<Receipt>;

    /// Get a pointer from the storage.
    async fn get_pointer(
        &self,
        kind: RegistryKind,
        address: &Address,
        name: &str,
        version: &Version,
    ) -> Result<Option<PackagePointer>>;
}
