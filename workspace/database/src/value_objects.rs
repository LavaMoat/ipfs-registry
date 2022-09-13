use semver::{BuildMetadata, Prerelease, Version};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{format_description, OffsetDateTime, PrimitiveDateTime};
use web3_address::ethereum::Address;

use cid::Cid;
use ipfs_registry_core::{Namespace, PackageName};

use sqlx::{sqlite::SqliteRow, FromRow, Row};

use crate::Result;

pub(crate) fn parse_date_time(date_time: &str) -> Result<OffsetDateTime> {
    let format = format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second]",
    )?;
    Ok(PrimitiveDateTime::parse(date_time, &format)?.assume_utc())
}

/// Collection of records with associated total row count.
#[derive(Debug, Serialize, Deserialize)]
pub struct ResultSet<T> {
    pub records: Vec<T>,
    pub count: i64,
}

impl<T> ResultSet<T> {
    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn is_zero(&self) -> bool {
        self.is_empty() && self.count == 0
    }
}

/// Convert into a result set.
pub trait IntoResultSet<T, R> {
    fn into_result_set(self) -> ResultSet<R>;
}

impl IntoResultSet<Vec<PackageRecord>, PackageRecord> for Vec<PackageRecord> {
    fn into_result_set(self) -> ResultSet<PackageRecord> {
        let count = if self.is_empty() {
            0
        } else {
            self.get(0).unwrap().count
        };
        ResultSet {
            records: self,
            count,
        }
    }
}

impl IntoResultSet<Vec<VersionRecord>, VersionRecord> for Vec<VersionRecord> {
    fn into_result_set(self) -> ResultSet<VersionRecord> {
        let count = if self.is_empty() {
            0
        } else {
            self.get(0).unwrap().count
        };
        ResultSet {
            records: self,
            count,
        }
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
pub struct UserRecord {
    /// Namespace foreign key.
    #[serde(skip)]
    pub namespace_id: i64,
    /// Publisher foreign key.
    #[serde(skip)]
    pub publisher_id: i64,
    /// Address of the publisher.
    pub address: Address,
}

impl FromRow<'_, SqliteRow> for UserRecord {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        let namespace_id: i64 = row.try_get("namespace_id")?;
        let publisher_id: i64 = row.try_get("publisher_id")?;
        let address: Vec<u8> = row.try_get("address")?;

        let address: [u8; 20] = address
            .as_slice()
            .try_into()
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let address: Address = address.into();

        Ok(Self {
            namespace_id,
            publisher_id,
            address,
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
    pub name: PackageName,
    /// Creation date and time.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// Collection of versions.
    #[serde(skip_serializing_if = "ResultSet::is_zero")]
    pub versions: ResultSet<VersionRecord>,
    /// Count of total rows.
    #[serde(skip)]
    pub count: i64,
}

impl FromRow<'_, SqliteRow> for PackageRecord {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        let namespace_id: i64 = row.try_get("namespace_id")?;
        let package_id: i64 = row.try_get("package_id")?;
        let name: String = row.try_get("name")?;
        let created_at: String = row.try_get("created_at")?;

        let name: PackageName =
            name.parse().map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let created_at = parse_date_time(&created_at)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let count = if let Ok(count) = row.try_get::<i64, _>("count") {
            count
        } else {
            0
        };

        Ok(Self {
            namespace_id,
            package_id,
            name,
            created_at,
            versions: ResultSet::<VersionRecord> {
                records: vec![],
                count: 0,
            },
            count,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<Value>,
    /// Content identifier.
    #[serde(
        skip_serializing_if = "Option::is_none")]
    pub content_id: Option<String>,
    /// Pointer identifier.
    pub pointer_id: String,
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

    /// Yanked message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yanked: Option<String>,

    /// Count of total rows.
    #[serde(skip)]
    pub count: i64,
}

impl VersionRecord {
    /// Parse a content id.
    pub fn parse_cid(&self) -> Result<Option<Cid>> {
        if let Some(cid) = &self.content_id {
            let cid: Cid = cid.parse()?;
            Ok(Some(cid))
        } else { Ok(None) }
    }
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

        let content_id: Option<String> = row.try_get("content_id")?;
        let pointer_id: String = row.try_get("pointer_id")?;

        let signature: Vec<u8> = row.try_get("signature")?;
        let checksum: Vec<u8> = row.try_get("checksum")?;

        let created_at: String = row.try_get("created_at")?;

        let yanked: Option<String> = row.try_get("yanked")?;

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

        let package = if let Ok(package) = row.try_get::<String, _>("package")
        {
            let package: Value = serde_json::from_str(&package)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
            Some(package)
        } else {
            None
        };

        /*
        let content_id = if let Some(cid) = content_id {
            let cid: Cid =
                cid.parse().map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
            Some(cid)
        } else {
            None
        };
        */

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

        let count = if let Ok(count) = row.try_get::<i64, _>("count") {
            count
        } else {
            0
        };

        Ok(Self {
            publisher_id,
            version_id,
            package_id,
            content_id,
            pointer_id,
            version,
            package,
            signature,
            checksum,
            created_at,
            yanked,
            count,
        })
    }
}
