use std::path::PathBuf;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum Error {
    /// Error generated when a path is not a file.
    #[error("path {0} is not a file")]
    NotFile(PathBuf),

    /// Error generated when a path is not a directory.
    #[error("not a directory {0}")]
    NotDirectory(PathBuf),

    /// Error generated when a host is invalid.
    #[error("host for URL {0} is invalid")]
    InvalidHost(Url),

    /// Error generated when a port is invalid.
    #[error("port for URL {0} is invalid")]
    InvalidPort(Url),

    /// Error generated when a scheme is not recognised.
    #[error("not a recognised scheme {0}")]
    InvalidScheme(String),

    /// Error generated when an AWS configuration is expected.
    #[error("configuration is invalid, expecting AWS config")]
    InvalidAwsConfig,

    /// Error generated by the io module.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Error generated by the address library.
    #[error(transparent)]
    Address(#[from] web3_address::Error),

    /// Error generated deserializing from TOML.
    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    /// Error generated by the JSON library.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Error generated attempting to parse a socket address.
    #[error(transparent)]
    AddrParse(#[from] std::net::AddrParseError),

    /// Error generated when a header value is invalid.
    #[error(transparent)]
    HeaderValue(#[from] axum::http::header::InvalidHeaderValue),

    /// Error generated when by the HTTP library.
    #[error(transparent)]
    Http(#[from] axum::http::Error),

    /// Error generated by the IPFS library.
    #[error(transparent)]
    Ipfs(#[from] ipfs_api_backend_hyper::Error),

    /// Error generated parsing MIME type.
    #[error(transparent)]
    Mime(#[from] mime::FromStrError),

    /// Error generated converting from a slice.
    #[error(transparent)]
    TryFromSlice(#[from] std::array::TryFromSliceError),

    /// Error generate by the ECDSA library.
    #[error(transparent)]
    Ecdsa(#[from] k256::ecdsa::Error),

    #[error(transparent)]
    Tls(#[from] rusoto_core::request::TlsError),

    //#[error(transparent)]
    //ParseRegion(#[from] rusoto_signature::region::ParseRegionError),
    #[error(transparent)]
    Credentials(#[from] rusoto_core::credential::CredentialsError),

    #[error(transparent)]
    GetObject(#[from] rusoto_core::RusotoError<rusoto_s3::GetObjectError>),

    #[error(transparent)]
    HeadBucket(#[from] rusoto_core::RusotoError<rusoto_s3::HeadBucketError>),

    #[error(transparent)]
    PutObject(#[from] rusoto_core::RusotoError<rusoto_s3::PutObjectError>),

    #[error(transparent)]
    ParseRegion(#[from] rusoto_signature::region::ParseRegionError),
}
