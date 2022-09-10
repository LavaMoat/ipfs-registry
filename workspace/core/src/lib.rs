mod error;
mod package;
mod tarball;

pub use error::Error;
pub use package::{
    Artifact, Definition, Namespace, ObjectKey, PackageKey, PackageMeta,
    PackageReader, PackageSignature, Pointer, Receipt, RegistryKind,
};

pub type Result<T> = std::result::Result<T, error::Error>;

/// Name of the header used for signatures.
pub const X_SIGNATURE: &str = "x-signature";

/// Well known message used for self-signing.
pub const WELL_KNOWN_MESSAGE: &[u8] = b".ipfs-registry";
