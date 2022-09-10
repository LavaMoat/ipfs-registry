use ipfs_registry_core::Namespace;
use semver::Version;
use thiserror::Error;
use web3_address::ethereum::Address;

#[derive(Debug, Error)]
pub enum Error {
    #[error("package {0}/{1}/{2} already exists")]
    PackageExists(Namespace, String, Version),

    #[error("publisher {0} is not authorized")]
    Unauthorized(Address),

    #[error("unknown publisher {0}")]
    UnknownPublisher(Address),

    #[error("unknown namespace {0}")]
    UnknownNamespace(Namespace),

    #[error("failed to fetch record {0} after insert")]
    InsertFetch(i64),

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
