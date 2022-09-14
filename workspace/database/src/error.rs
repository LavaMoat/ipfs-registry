use ipfs_registry_core::{Namespace, PackageName};
use semver::Version;
use thiserror::Error;
use web3_address::ethereum::Address;

#[derive(Debug, Error)]
pub enum Error {
    #[error("package {0}/{1}/{2} already exists")]
    PackageExists(Namespace, PackageName, Version),

    #[error("publisher {0} is not authorized")]
    Unauthorized(Address),

    #[error("unknown publisher {0}")]
    UnknownPublisher(Address),

    #[error("user {0} already exists in {1}")]
    UserExists(Address, String),

    #[error("unknown namespace {0}")]
    UnknownNamespace(Namespace),

    #[error("unknown package {0}")]
    UnknownPackage(String),

    #[error("version {0} is not ahead of latest {1}")]
    VersionNotAhead(Version, Version),

    #[error("failed to fetch record {0} after insert")]
    InsertFetch(i64),

    #[error("could not find a version for a package")]
    NoPackageVersion,

    #[error(transparent)]
    Core(#[from] ipfs_registry_core::Error),

    /// Error generated converting from a slice.
    #[error(transparent)]
    TryFromSlice(#[from] std::array::TryFromSliceError),

    #[error(transparent)]
    Semver(#[from] semver::Error),

    #[error(transparent)]
    Address(#[from] web3_address::Error),

    #[error(transparent)]
    Sql(#[from] sqlx::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Cid(#[from] cid::Error),

    #[error(transparent)]
    FormatDescription(#[from] time::error::InvalidFormatDescription),

    #[error(transparent)]
    TimeParse(#[from] time::error::Parse),
}
