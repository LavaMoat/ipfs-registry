//! Database model and value objects.
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod error;
mod model;
mod value_objects;

pub use error::Error;

/// Result type for the database library.
pub type Result<T> = std::result::Result<T, Error>;

pub use model::*;
pub use value_objects::*;
