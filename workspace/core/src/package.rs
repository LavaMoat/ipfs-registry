//! Types for package definitions.
use cid::Cid;
use semver::Version;
use serde::{
    de::{self, Deserializer, Visitor},
    ser::Serializer,
    Deserialize, Serialize,
};
use serde_json::Value;
use std::{fmt, str::FromStr};
use web3_address::ethereum::Address;

use crate::{
    tarball::{decompress, read_npm_package},
    Error, Result,
};

const IPFS_DELIMITER: &str = "/ipfs/";

/// Validate a namespace or package name.
pub fn validate(s: &str) -> bool {
    let invalid = "/\\ \t\n@:";
    for c in invalid.chars() {
        if s.find(c).is_some() {
            return false;
        }
    }
    true
}

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Namespace(String);

impl Namespace {
    /// Create a new namespace without checking the source is valid.
    pub fn new_unchecked(s: &str) -> Self {
        Self(s.to_owned())
    }

    /// Get a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get a reference to the underlying bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Namespace {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if validate(s) {
            Ok(Namespace(s.to_owned()))
        } else {
            Err(Error::InvalidNamespace(s.to_owned()))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackageName(String);

impl PackageName {
    /// Create a new package name without checking the source is valid.
    pub fn new_unchecked(s: &str) -> Self {
        Self(s.to_owned())
    }

    /// Get a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /*
    /// Get a reference to the underlying bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
    */
}

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PackageName {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if validate(s) {
            Ok(PackageName(s.to_owned()))
        } else {
            Err(Error::InvalidPackageName(s.to_owned()))
        }
    }
}

/// Reference to a package artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackageKey {
    /// Direct artifact reference using an IPFS content identifier.
    Cid(Cid),
    /// Pointer reference by namespace, package name and version.
    Pointer(Namespace, PackageName, Version),
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

            let namespace: Namespace = parts.remove(0).parse()?;
            let name: PackageName = parts.remove(0).parse()?;
            let version = parts.remove(0);
            let version: Version = Version::parse(version)?;

            Ok(Self::Pointer(namespace, name, version))
        }
    }
}

impl Serialize for PackageKey {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = self.to_string();
        serializer.serialize_str(&value)
    }
}

struct PackageKeyVisitor;

impl<'de> Visitor<'de> for PackageKeyVisitor {
    type Value = PackageKey;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string for package id")
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        let package_key: PackageKey = v.parse().unwrap();
        Ok(package_key)
    }
}

impl<'de> Deserialize<'de> for PackageKey {
    fn deserialize<D>(
        deserializer: D,
    ) -> std::result::Result<PackageKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(PackageKeyVisitor)
    }
}

/// Type that represents a reference to a file object.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectKey {
    /// Reference to an IPFS content identifier.
    Cid(Cid),
    /// Reference to a bucket key.
    Key(String),
}

impl FromStr for ObjectKey {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let result: Result<Cid> = s.try_into().map_err(Error::from);
        match result {
            Ok(cid) => Ok(ObjectKey::Cid(cid)),
            Err(_e) => Ok(ObjectKey::Key(s.to_owned())),
        }
    }
}

impl fmt::Display for ObjectKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cid(value) => write!(f, "{}", value),
            Self::Key(value) => write!(f, "{}", value),
        }
    }
}

/// Meta data extracted from a package definition file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageMeta {
    /// Name of the package.
    pub name: PackageName,
    /// Version of the package.
    pub version: Version,
}

/// Package descriptor in the context of a namespace.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Artifact {
    /// The kind of registry.
    pub kind: RegistryKind,
    /// Organization namespace.
    pub namespace: Namespace,
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
    /// Signature of the package file.
    #[serde(
        serialize_with = "hex::serde::serialize",
        deserialize_with = "hex::serde::deserialize"
    )]
    pub value: [u8; 65],
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
    /// Package identifier.
    pub id: PackageKey,
    /// Package descriptor.
    pub artifact: Artifact,
    /// Key for the IPFS package reference.
    pub key: Option<PackageKey>,
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
        if let PackageKey::Pointer(org, name, version) = package_key {
            assert_eq!(Namespace::new_unchecked("example.com"), org);
            assert_eq!(PackageName::new_unchecked("mock-package"), name);
            assert_eq!(Version::new(1, 0, 0), version);
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

    #[test]
    fn serde_package_key_ipfs() -> Result<()> {
        let key = "/ipfs/bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
        let package_key: PackageKey = key.parse()?;
        let serialized = serde_json::to_string(&package_key)?;
        let deserialized: PackageKey = serde_json::from_str(&serialized)?;
        assert_eq!(package_key, deserialized);
        Ok(())
    }

    #[test]
    fn serde_package_key_path() -> Result<()> {
        let key = "example.com/mock-package/1.0.0";
        let package_key: PackageKey = key.parse()?;
        let serialized = serde_json::to_string(&package_key)?;
        let deserialized: PackageKey = serde_json::from_str(&serialized)?;
        assert_eq!(package_key, deserialized);
        Ok(())
    }
}
