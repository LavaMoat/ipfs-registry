use ipfs_registry_core::{Namespace, PackageKey, PackageName};
use semver::Version;
use std::fmt;
use thiserror::Error;
use web3_address::ethereum::Address;

/// Enumeration of the identifiers that can trigger
/// a not found error.
#[derive(Debug)]
pub enum NotFound {
    /// User not found.
    User(Address),
    /// Namespace not found.
    Namespace(Namespace),
    /// Package not found by name.
    PackageName(PackageName),
    /// Package not found by key.
    PackageKey(PackageKey),
}

impl fmt::Display for NotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User(value) => write!(f, "user {} not found", value),
            Self::Namespace(value) => {
                write!(f, "namespace {} not found", value)
            }
            Self::PackageName(value) => {
                write!(f, "package {} not found", value)
            }
            Self::PackageKey(value) => {
                write!(f, "package key {} not found", value)
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("package {0}/{1}/{2} already exists")]
    PackageExists(Namespace, PackageName, Version),

    #[error("publisher {0} is not authorized")]
    Unauthorized(Address),

    #[error("user {0} already exists in {1}")]
    UserExists(Address, String),

    #[error("{0}")]
    NotFound(NotFound),

    #[error("user {0} already has access to {1}")]
    AccessRestrictionExists(Address, PackageName),

    #[error("user {0} does not have access to {1}")]
    AccessRestrictionMissing(Address, PackageName),

    #[error("version {0} is not ahead of latest {1}")]
    VersionNotAhead(Version, Version),

    #[error("failed to fetch record {0} after insert")]
    InsertFetch(i64),

    #[error("invalid sort order {0}")]
    InvalidSortOrder(String),

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
