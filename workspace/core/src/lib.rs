mod error;
mod package;
mod tarball;

pub use error::Error;
pub use package::{Definition, Descriptor, PackageReader, RegistryKind, Document};

pub type Result<T> = std::result::Result<T, error::Error>;

/// Name of the header used for signatures.
pub const X_SIGNATURE: &str = "x-signature";
