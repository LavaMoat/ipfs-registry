use cid::Cid;
use semver::Version;
use serde::Serialize;
use serde_json::Value;
use time::{format_description, OffsetDateTime, PrimitiveDateTime};
use web3_address::ethereum::Address;

use crate::Result;

pub(crate) fn parse_date_time(date_time: &str) -> Result<OffsetDateTime> {
    let format = format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second]",
    )?;
    Ok(PrimitiveDateTime::parse(date_time, &format)?.assume_utc())
}

#[derive(Debug, Serialize)]
pub struct PublisherRecord {
    /// Publisher primary key.
    pub publisher_id: i64,
    /// Address of the publisher.
    pub address: Address,
    /// Creation date and time.
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct NamespaceRecord {
    /// Namespace primary key.
    pub namespace_id: i64,
    /// Name for the namespace.
    pub name: String,
    /// Owner of the namespace.
    pub owner: Address,
    /// Additional publishers.
    pub publishers: Vec<Address>,
    /// Creation date and time.
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

#[derive(Debug, Serialize)]
pub struct PackageRecord {
    /// Namespace foreign key.
    pub namespace_id: i64,
    /// Package primary key.
    pub package_id: i64,
    /// Name of the package.
    pub name: String,
    /// Creation date and time.
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct VersionRecord {
    /// Publisher foreign key.
    pub publisher_id: i64,
    /// Package foreign key.
    pub package_id: i64,
    /// Version primary key.
    pub version_id: i64,
    /// Version of the package.
    pub version: Version,
    /// Package meta data.
    pub package: Value,
    /// Content identifier.
    pub content_id: Option<Cid>,
    /// Creation date and time.
    pub created_at: OffsetDateTime,
}
