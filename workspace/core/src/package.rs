//! Types for package definitions.
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use web3_address::ethereum::Address;

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

/// Type that represents a reference to a file object.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectKey {
    /// Reference to an IPFS content identifier.
    Cid(String),
    /// Reference to a bucket key.
    Key(String),
}

impl AsRef<str> for ObjectKey {
    fn as_ref(&self) -> &str {
        match self {
            Self::Cid(value) => value,
            Self::Key(value) => value,
        }
    }
}

/// Meta data extracted from a package definition file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageMeta {
    /// Name of the package.
    pub name: String,
    /// Version of the package.
    pub version: Version,
}

/// Package descriptor in the context of a namespace.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Artifact {
    /// The kind of registry.
    pub kind: RegistryKind,
    /// Organization namespace.
    pub namespace: String,
    /// Package descriptor.
    pub package: PackageMeta,
}

/// Definition of a package.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Definition {
    /// The id of the package archive.
    pub object: ObjectKey,
    /// Package descriptor.
    pub artifact: Artifact,
    /// Signature of the package.
    pub signature: PackageSignature,
    /// SHA3-256 checksum of the package file.
    #[serde(
        serialize_with = "hex::serde::serialize",
        deserialize_with = "hex::serde::deserialize"
    )]
    pub checksum: Vec<u8>,
}

/// Package signature and address of the verifying key.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageSignature {
    /// Address of the signer.
    pub signer: Address,
    /// Signature of the package file encoded as base64.
    pub value: String,
}

/// Type that points to a package archive and wraps the meta
/// data extracted from the archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pointer {
    /// The package definition.
    pub definition: Definition,
    /// Package meta data extracted from the archive (eg: package.json).
    pub package: Value,
}

/// Receipt for a published package.
#[derive(Debug, Serialize, Deserialize)]
pub struct Receipt {
    /// Collection of pointers, one for each storage layer.
    pub pointers: Vec<ObjectKey>,
    /// Package descriptor.
    pub artifact: Artifact,
}

/// Read a descriptor from a package.
pub struct PackageReader;

impl PackageReader {
    /// Read a descriptor from file content.
    pub fn read(
        kind: RegistryKind,
        buffer: &[u8],
    ) -> Result<(PackageMeta, Value)> {
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
