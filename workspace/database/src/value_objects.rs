use cid::Cid;
use semver::Version;
use serde_json::Value;
use web3_address::ethereum::Address;

#[derive(Debug)]
pub struct PublisherRecord {
    /// Publisher primary key.
    pub publisher_id: i64,
    /// Address of the publisher.
    pub address: Address,
    /// Creation date and time.
    pub created_at: String,
}

#[derive(Debug)]
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
    pub created_at: String,
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
pub struct PackageRecord {
    /// Namespace foreign key.
    pub namespace_id: i64,
    /// Package primary key.
    pub package_id: i64,
    /// Name of the package.
    pub name: String,
}

#[derive(Debug)]
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
    pub created_at: String,
}
