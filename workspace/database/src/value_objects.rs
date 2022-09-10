use cid::Cid;
use semver::Version;
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

#[derive(Debug)]
pub(crate) struct PublisherRow {
    /// Publisher primary key.
    pub publisher_id: i64,
    /// Address of the publisher.
    pub address: Vec<u8>,
    /// Creation date and time.
    pub created_at: String,
}

impl TryFrom<PublisherRow> for PublisherRecord {
    type Error = Error;

    fn try_from(
        row: PublisherRow,
    ) -> std::result::Result<PublisherRecord, Self::Error> {
        let created_at = parse_date_time(&row.created_at)?;
        let address: [u8; 20] = row.address.as_slice().try_into()?;
        let address: Address = address.into();
        Ok(PublisherRecord {
            publisher_id: row.publisher_id,
            address,
            created_at,
        })
    }
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

#[derive(Debug)]
pub(crate) struct NamespaceRow {
    /// Namespace primary key.
    pub namespace_id: i64,
    /// Publisher foreign key.
    pub publisher_id: i64,
    /// Name for the namespace.
    pub name: String,
    /// Address of the owner.
    pub address: Vec<u8>,
    /// Creation date and time.
    pub created_at: String,
}

impl TryFrom<NamespaceRow> for NamespaceRecord {
    type Error = Error;

    fn try_from(
        row: NamespaceRow,
    ) -> std::result::Result<NamespaceRecord, Self::Error> {
        let created_at = parse_date_time(&row.created_at)?;

        let owner: [u8; 20] = row.address.as_slice().try_into()?;
        let owner: Address = owner.into();

        Ok(NamespaceRecord {
            namespace_id: row.namespace_id,
            owner,
            name: row.name.parse()?,
            created_at,
            publishers: Vec::new(),
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

#[derive(Debug)]
pub(crate) struct PackageRow {
    /// Namespace foreign key.
    pub namespace_id: i64,
    /// Package primary key.
    pub package_id: i64,
    /// Name of the package.
    pub name: String,
    /// Creation date and time.
    pub created_at: String,
}

impl TryFrom<PackageRow> for PackageRecord {
    type Error = Error;

    fn try_from(
        row: PackageRow,
    ) -> std::result::Result<PackageRecord, Self::Error> {
        // Parse to time type
        let created_at = parse_date_time(&row.created_at)?;
        Ok(PackageRecord {
            namespace_id: row.namespace_id,
            package_id: row.package_id,
            name: row.name,
            created_at,
        })
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

#[derive(Debug)]
pub(crate) struct VersionRow {
    /// Publisher foreign key.
    pub publisher_id: i64,
    /// Package foreign key.
    pub package_id: i64,
    /// Version primary key.
    pub version_id: i64,
    /// Version of the package.
    pub version: String,
    /// Package meta data.
    pub package: String,
    /// Content identifier.
    pub content_id: String,
    /// Package archive signature.
    pub signature: Vec<u8>,
    /// Archive checksum.
    pub checksum: Vec<u8>,
    /// Creation date and time.
    pub created_at: String,
}

impl TryFrom<VersionRow> for VersionRecord {
    type Error = Error;

    fn try_from(
        row: VersionRow,
    ) -> std::result::Result<VersionRecord, Self::Error> {
        let version: Version = Version::parse(&row.version)?;
        let package: Value = serde_json::from_str(&row.package)?;
        let content_id = row.content_id.parse()?;

        let signature: [u8; 65] = row.signature.as_slice().try_into()?;
        let checksum: [u8; 32] = row.checksum.as_slice().try_into()?;

        // Parse to time type
        let created_at = parse_date_time(&row.created_at)?;

        Ok(VersionRecord {
            publisher_id: row.publisher_id,
            version_id: row.version_id,
            package_id: row.package_id,
            content_id,
            version,
            package,
            signature,
            checksum,
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
