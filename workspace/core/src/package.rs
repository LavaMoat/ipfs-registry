//! Types for package definitions.
use std::fmt;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    tarball::{decompress, read_npm_package},
    Result,
};

/// Kinds or supported registries.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegistryKind {
    /// NPM compatible packages.
    Npm,
}
impl fmt::Display for RegistryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Npm => "npm",
            }
        )
    }
}

impl RegistryKind {
    /// Get the document name for this kind of registry.
    pub fn document_name(&self) -> &str {
        match self {
            Self::Npm => "package.json",
        }
    }
}

/// Describes a package.
#[derive(Debug, Serialize, Deserialize)]
pub struct Descriptor {
    pub name: String,
    pub version: Version,
}

/// Definition of a package.
#[derive(Debug, Serialize, Deserialize)]
pub struct Definition {
    /// The IPFS hash (cid) for the package blob.
    pub cid: String,
    /// Package descriptor.
    pub descriptor: Descriptor,
    /// Signature of the package file encoded as base64.
    pub signature: String,
}

/// Type that defines a package and it's associated 
/// meta data and cryptographic signature.
#[derive(Debug, Serialize, Deserialize)]
pub struct PackagePointer {
    /// The package definition.
    pub definition: Definition,
    /// Package meta data extracted from the archive (eg: package.json).
    pub package: Value,
}

/// Receipt for a published package.
#[derive(Debug, Serialize, Deserialize)]
pub struct PublishReceipt {
    /// The `cid` of the pointer.
    pub pointer: String,
    /// The `cid` of the package file.
    pub package: String,
    /// Signature of the package file encoded as base64.
    pub signature: String,
}

/// Read a descriptor from a package.
pub struct PackageReader;

impl PackageReader {
    /// Read a descriptor from file content.
    pub fn read(
        kind: RegistryKind,
        buffer: &[u8],
    ) -> Result<(Descriptor, Value)> {
        match kind {
            RegistryKind::Npm => {
                let contents = decompress(buffer)?;
                let (descriptor, buffer) = read_npm_package(&contents)?;
                let value: Value = serde_json::from_slice(buffer)?;
                Ok((descriptor, value))
            }
        }
    }
}
