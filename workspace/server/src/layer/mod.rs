//! Traits and types for storage layers.
use async_trait::async_trait;
use axum::body::Bytes;
use web3_address::ethereum::Address;

use ipfs_registry_core::{
    Descriptor, NamespacedDescriptor, PackagePointer, Receipt, RegistryKind,
};

use serde_json::Value;

use crate::Result;

pub(crate) mod ipfs;
pub(crate) mod s3;

pub(crate) const ROOT: &str = "ipkg-registry";
pub(crate) const NAME: &str = "pointer.json";
pub(crate) const BLOB: &str = "package.tgz";

/// Type for a storage and mirror layer.
pub(crate) struct Layers {
    pub storage: Box<dyn Layer + Send + Sync + 'static>,
    pub mirror: Option<Box<dyn Layer + Send + Sync + 'static>>,
}

#[async_trait]
impl Layer for Layers {
    async fn add_blob(
        &self,
        data: Bytes,
        descriptor: &NamespacedDescriptor,
    ) -> Result<String> {
        if let Some(mirror) = &self.mirror {
            let id = self.storage.add_blob(data.clone(), descriptor).await?;
            mirror.add_blob(data, descriptor).await?;
            Ok(id)
        } else {
            self.storage.add_blob(data, descriptor).await
        }
    }

    async fn get_blob(&self, id: &str) -> Result<Vec<u8>> {
        self.storage.get_blob(id).await
    }

    async fn add_pointer(
        &self,
        signature: String,
        address: &Address,
        descriptor: NamespacedDescriptor,
        archive_id: String,
        package: Value,
    ) -> Result<Receipt> {
        if let Some(mirror) = &self.mirror {
            let receipt = self
                .storage
                .add_pointer(
                    signature.clone(),
                    address,
                    descriptor.clone(),
                    archive_id.clone(),
                    package.clone(),
                )
                .await?;
            mirror
                .add_pointer(
                    signature, address, descriptor, archive_id, package,
                )
                .await?;
            Ok(receipt)
        } else {
            self.storage
                .add_pointer(
                    signature, address, descriptor, archive_id, package,
                )
                .await
        }
    }

    async fn get_pointer(
        &self,
        descriptor: &NamespacedDescriptor,
    ) -> Result<Option<PackagePointer>> {
        self.storage.get_pointer(descriptor).await
    }
}

/// Trait for a storage layer.
#[async_trait]
pub trait Layer {
    /// Add a blob to the storage and return an identifier.
    async fn add_blob(
        &self,
        data: Bytes,
        descriptor: &NamespacedDescriptor,
    ) -> Result<String>;

    /// Get a blob from storage by identifier.
    async fn get_blob(&self, id: &str) -> Result<Vec<u8>>;

    /// Add a pointer to the storage.
    async fn add_pointer(
        &self,
        signature: String,
        address: &Address,
        descriptor: NamespacedDescriptor,
        archive_id: String,
        package: Value,
    ) -> Result<Receipt>;

    /// Get a pointer from the storage.
    async fn get_pointer(
        &self,
        descriptor: &NamespacedDescriptor,
    ) -> Result<Option<PackagePointer>>;
}
