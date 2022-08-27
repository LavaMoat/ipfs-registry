//! Types for package definitions.
use semver::Version;
use serde::{Deserialize, Serialize};

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
