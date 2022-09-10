mod client;
mod commands;
mod error;
mod helpers;
mod input;

pub type Result<T> = std::result::Result<T, error::Error>;

pub use error::Error;

pub use client::RegistryClient;
pub use commands::*;
