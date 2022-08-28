//! Types for package definitions.
use std::fmt;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{Result, tarball::{decompress, read_npm_package}};

/// Kinds or supported registries.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegistryKind {
    /// NPM compatible packages.
    Npm,
}
impl fmt::Display for RegistryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Npm => "npm",
        })
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
    pub fn read(kind: RegistryKind, buffer: &[u8]) -> Result<Descriptor> {
        match kind {
            RegistryKind::Npm => {
                let contents = decompress(buffer)?;
                read_npm_package(&contents) 
            }
        }
    }
}
