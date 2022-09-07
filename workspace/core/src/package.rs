//! Types for package definitions.
use cid::Cid;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, str::FromStr};
use web3_address::ethereum::Address;

use crate::{
    tarball::{decompress, read_npm_package},
    Error, Result,
};

const IPFS_DELIMITER: &str = "/ipfs/";

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

/// Reference to a package artifact.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageKey {
    /// Direct artifact reference using an IPFS content identifier.
    Cid(Cid),
    /// Pointer reference by namespace, package name and version.
    Pointer(String, String, Version),
}

impl fmt::Display for PackageKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cid(cid) => write!(f, "{}{}", IPFS_DELIMITER, cid),
            Self::Pointer(org, name, version) => {
                write!(f, "{}/{}/{}", org, name, version)
            }
        }
    }
}

impl FromStr for PackageKey {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let hash = match s.find(IPFS_DELIMITER) {
            Some(index) => {
                if index == 0 {
                    Some(&s[IPFS_DELIMITER.len()..])
                } else {
                    None
                }
            }
            None => None,
        };

        if let Some(hash) = hash {
            let cid: Cid = hash.try_into()?;
            Ok(Self::Cid(cid))
        } else {
            let mut parts: Vec<&str> = s.split('/').collect();
            if parts.len() != 3 {
                return Err(Error::InvalidPath(s.to_owned()));
            }

            let org = parts.remove(0);
            let name = parts.remove(0);
            let version = parts.remove(0);
            let version: Version = Version::parse(version)?;

            Ok(Self::Pointer(org.to_owned(), name.to_owned(), version))
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use semver::Version;

    #[test]
    fn parse_package_key_ipfs() -> Result<()> {
        let key = "/ipfs/bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
        let package_key: PackageKey = key.parse()?;
        if let PackageKey::Cid(cid) = package_key {
            assert_eq!(cid::Version::V1, cid.version());
            assert_eq!(112, cid.codec());
            Ok(())
        } else {
            panic!("expecting CID for package key");
        }
    }

    #[test]
    fn parse_package_key_path() -> Result<()> {
        let key = "example.com/mock-package/1.0.0";
        let package_key: PackageKey = key.parse()?;
        if let PackageKey::Pointer(org, name, version) = &package_key {
            assert_eq!("example.com", org);
            assert_eq!("mock-package", name);
            assert_eq!(&Version::new(1, 0, 0), version);
            Ok(())
        } else {
            panic!("expecting path for package key");
        }
    }

    #[test]
    fn parse_package_error() -> Result<()> {
        // Missing CID hash
        let key = "/ipfs/";
        let result = key.parse::<PackageKey>();
        assert!(result.is_err());

        // Bad path
        let key = "example.com";
        let result = key.parse::<PackageKey>();
        assert!(result.is_err());

        // Too many parts (leading slash)
        let key = "/a/b/c";
        let result = key.parse::<PackageKey>();
        assert!(result.is_err());

        // Too many parts (trailing slash)
        let key = "a/b/c/";
        let result = key.parse::<PackageKey>();
        assert!(result.is_err());

        // Invalid semver
        let key = "a/b/foo";
        let result = key.parse::<PackageKey>();
        assert!(result.is_err());

        Ok(())
    }
}
