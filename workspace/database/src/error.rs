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

/// Errors thrown by the database library.
#[derive(Debug, Error)]
pub enum Error {
    /// Error generated when a package already exists.
    #[error("package {0}/{1}/{2} already exists")]
    PackageExists(Namespace, PackageName, Version),

    /// Error generated when a user is not authorized.
    #[error("user {0} is not authorized")]
    Unauthorized(Address),

    /// Error generated when a user already exists.
    #[error("user {0} already exists in {1}")]
    UserExists(Address, String),

    /// Error generated when a resource could not be found.
    #[error("{0}")]
    NotFound(NotFound),

    /// Error generated when an access restriction already exists.
    #[error("user {0} already has access to {1}")]
    AccessRestrictionExists(Address, PackageName),

    /// Error generated when an access restriction does not exist.
    #[error("user {0} does not have access to {1}")]
    AccessRestrictionMissing(Address, PackageName),

    /// Error generated when a version is not ahead of the latest version.
    #[error("version {0} is not ahead of latest {1}")]
    VersionNotAhead(Version, Version),

    /// Error generated if fetching a record fails immediately after insertion.
    #[error("failed to fetch record {0} after insert")]
    InsertFetch(i64),

    /// Error generated when a sort order is invalid.
    #[error("invalid sort order {0}")]
    InvalidSortOrder(String),

    /// Error generated when a version includes variant is invalid.
    #[error("invalid version includes {0}")]
    InvalidVersionIncludes(String),

    /// Error generated when the a version for a package could not be found.
    #[error("could not find a version for a package")]
    NoPackageVersion,

    /// Error generated by the core library.
    #[error(transparent)]
    Core(#[from] ipfs_registry_core::Error),

    /// Error generated converting from a slice.
    #[error(transparent)]
    TryFromSlice(#[from] std::array::TryFromSliceError),

    /// Error generated by the semver library.
    #[error(transparent)]
    Semver(#[from] semver::Error),

    /// Error generated by the address library.
    #[error(transparent)]
    Address(#[from] web3_address::Error),

    /// Error generated by the SQL library.
    #[error(transparent)]
    Sql(#[from] sqlx::Error),

    /// Error generated by the JSON library.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Error generated by the CID library.
    #[error(transparent)]
    Cid(#[from] cid::Error),

    /// Error generated by the time library when formatting descriptions.
    #[error(transparent)]
    FormatDescription(#[from] time::error::InvalidFormatDescription),

    /// Error generated by the time library when parsing.
    #[error(transparent)]
    TimeParse(#[from] time::error::Parse),
}
