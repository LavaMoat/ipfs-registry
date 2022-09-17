//! Library for the package registry executable.
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod error;

pub use error::Error;

/// Result type for the executable library.
pub type Result<T> = std::result::Result<T, Error>;
