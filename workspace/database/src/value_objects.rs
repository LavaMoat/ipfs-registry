use semver::{BuildMetadata, Prerelease, Version};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{format_description, OffsetDateTime, PrimitiveDateTime};
use web3_address::ethereum::Address;

use ipfs_registry_core::{Namespace, ObjectKey};

use sqlx::{sqlite::SqliteRow, FromRow, Row};

use crate::{Error, Result};

pub(crate) fn parse_date_time(date_time: &str) -> Result<OffsetDateTime> {
    let format = format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second]",
    )?;
    Ok(PrimitiveDateTime::parse(date_time, &format)?.assume_utc())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublisherRecord {
    /// Publisher primary key.
    #[serde(skip)]
    pub publisher_id: i64,
    /// Address of the publisher.
    pub address: Address,
    /// Creation date and time.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl FromRow<'_, SqliteRow> for PublisherRecord {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        let publisher_id: i64 = row.try_get("publisher_id")?;
        let address: Vec<u8> = row.try_get("address")?;
        let created_at: String = row.try_get("created_at")?;

        let address: [u8; 20] = address
            .as_slice()
            .try_into()
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let address: Address = address.into();

        let created_at = parse_date_time(&created_at)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        Ok(Self {
            publisher_id,
            address,
            created_at,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NamespaceRecord {
    /// Namespace primary key.
    #[serde(skip)]
    pub namespace_id: i64,
    /// Name for the namespace.
    pub name: Namespace,
    /// Owner of the namespace.
    pub owner: Address,
    /// Additional publishers.
    pub publishers: Vec<Address>,
    /// Creation date and time.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl FromRow<'_, SqliteRow> for NamespaceRecord {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        let namespace_id: i64 = row.try_get("namespace_id")?;
        //let publisher_id: i64 = row.try_get("publisher_id")?;
        let name: String = row.try_get("name")?;
        let address: Vec<u8> = row.try_get("address")?;
        let created_at: String = row.try_get("created_at")?;

        let name: Namespace =
            name.parse().map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let address: [u8; 20] = address
            .as_slice()
            .try_into()
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let address: Address = address.into();

        let created_at = parse_date_time(&created_at)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        Ok(Self {
            namespace_id,
            //publisher_id,
            publishers: vec![],
            name,
            owner: address,
            created_at,
        })
    }
}

impl NamespaceRecord {
    /// Determine if an address is allowed to publish to
    /// this namespace.
    pub fn can_publish(&self, address: &Address) -> bool {
        if &self.owner == address {
            true
        } else {
            self.publishers.iter().any(|a| a == address)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageRecord {
    /// Namespace foreign key.
    #[serde(skip)]
    pub namespace_id: i64,
    /// Package primary key.
    #[serde(skip)]
    pub package_id: i64,
    /// Name of the package.
    pub name: String,
    /// Creation date and time.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl FromRow<'_, SqliteRow> for PackageRecord {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        let namespace_id: i64 = row.try_get("namespace_id")?;
        let package_id: i64 = row.try_get("package_id")?;
        let name: String = row.try_get("name")?;
        let created_at: String = row.try_get("created_at")?;

        let created_at = parse_date_time(&created_at)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        Ok(Self {
            namespace_id,
            package_id,
            name,
            created_at,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionRecord {
    /// Publisher foreign key.
    #[serde(skip)]
    pub publisher_id: i64,
    /// Package foreign key.
    #[serde(skip)]
    pub package_id: i64,
    /// Version primary key.
    #[serde(skip)]
    pub version_id: i64,
    /// Version of the package.
    pub version: Version,
    /// Package meta data.
    pub package: Value,
    /// Content identifier.
    pub content_id: ObjectKey,
    /// Package archive signature.
    #[serde(
        serialize_with = "hex::serde::serialize",
        deserialize_with = "hex::serde::deserialize"
    )]
    pub signature: [u8; 65],
    /// Package archive checksum.
    #[serde(
        serialize_with = "hex::serde::serialize",
        deserialize_with = "hex::serde::deserialize"
    )]
    pub checksum: [u8; 32],
    /// Creation date and time.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl FromRow<'_, SqliteRow> for VersionRecord {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        let publisher_id: i64 = row.try_get("publisher_id")?;
        let version_id: i64 = row.try_get("version_id")?;
        let package_id: i64 = row.try_get("package_id")?;

        let major: i64 = row.try_get("major")?;
        let minor: i64 = row.try_get("minor")?;
        let patch: i64 = row.try_get("patch")?;

        let pre: Option<String> = row.try_get("pre")?;
        let build: Option<String> = row.try_get("build")?;

        let package: String = row.try_get("package")?;
        let content_id: String = row.try_get("content_id")?;

        let signature: Vec<u8> = row.try_get("signature")?;
        let checksum: Vec<u8> = row.try_get("checksum")?;

        let created_at: String = row.try_get("created_at")?;

        let mut version =
            Version::new(major as u64, minor as u64, patch as u64);
        if let Some(pre) = &pre {
            version.pre = Prerelease::new(pre)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        }
        if let Some(build) = &build {
            version.build = BuildMetadata::new(build)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        }

        let package: Value = serde_json::from_str(&package)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let content_id = content_id
            .parse()
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let signature: [u8; 65] = signature
            .as_slice()
            .try_into()
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let checksum: [u8; 32] = checksum
            .as_slice()
            .try_into()
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let created_at = parse_date_time(&created_at)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        Ok(Self {
            publisher_id,
            version_id,
            package_id,
            content_id,
            version,
            package,
            signature,
            checksum,
            created_at,
        })
    }
}
