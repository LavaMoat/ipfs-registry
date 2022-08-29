//! Types for package definitions.
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fmt;

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
}

/// Read a descriptor from a package.
pub struct PackageReader;

impl PackageReader {
    /// Read a descriptor from file content.
    pub fn read(
        kind: RegistryKind,
        buffer: &[u8],
    ) -> Result<(Descriptor, Vec<u8>)> {
        match kind {
            RegistryKind::Npm => {
                let contents = decompress(buffer)?;
                let (descriptor, buffer) = read_npm_package(&contents)?;
                Ok((descriptor, buffer.to_vec()))
            }
        }
    }
}
