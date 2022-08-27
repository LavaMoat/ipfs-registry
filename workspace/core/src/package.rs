//! Types for package definitions.
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use semver::Version;

/// Address computed from a user's public key.
type Address = [u8; 20];

/// Hash of a package name to represent a package.
type PackageId = [u8; 32];

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
    pub hash: String,
    /// Package descriptor.
    pub descriptor: Descriptor,
}

/// Collection of package versions.
#[derive(Default, Serialize, Deserialize)]
pub struct Package {
    versions: HashMap<Version, Definition>,
}

/// Packages for an account profile.
#[derive(Default)]
pub struct Profile {
    packages: HashMap<PackageId, Package>,
}

/// Index of all available packages.
#[derive(Default)]
pub struct Index {
    accounts: HashMap<Address, Profile>,
}
