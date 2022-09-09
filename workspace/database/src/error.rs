use semver::Version;
use thiserror::Error;
use web3_address::ethereum::Address;

#[derive(Debug, Error)]
pub enum Error {
    #[error("package {0}/{1}/{2} already exists")]
    PackageExists(String, String, Version),

    #[error("publisher {0} is not authorized")]
    Unauthorized(Address),

    #[error("unknown publisher {0}")]
    UnknownPublisher(Address),

    #[error("unknown namespace {0}")]
    UnknownNamespace(String),

    /// Error generated converting from a slice.
    #[error(transparent)]
    TryFromSlice(#[from] std::array::TryFromSliceError),

    #[error(transparent)]
    Semver(#[from] semver::Error),

    #[error(transparent)]
    Sql(#[from] sqlx::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Cid(#[from] cid::Error),
}
