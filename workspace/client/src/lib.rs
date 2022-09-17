//! Client implementation and commands.
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod client;
mod commands;
mod error;
mod helpers;
mod input;

/// Result type for the client library.
pub type Result<T> = std::result::Result<T, error::Error>;

pub use client::RegistryClient;
pub use commands::*;
pub use error::Error;
