use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    /// Error generated when a path is not a file.
    #[error("path {0} is not a file")]
    NotFile(PathBuf),

    /// Error generated when a path is not a directory.
    #[error("not a directory {0}")]
    NotDirectory(PathBuf),

    /// Error generated by the io module.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Error generated deserializing from TOML.
    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    /// Error generated attempting to parse a socket address.
    #[error(transparent)]
    AddrParse(#[from] std::net::AddrParseError),

    /// Error generated when a header value is invalid.
    #[error(transparent)]
    HeaderValue(#[from] axum::http::header::InvalidHeaderValue),
}
