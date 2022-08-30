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
    pub primary: Box<dyn Layer>,
    pub backup: Option<Box<dyn Layer>>,
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