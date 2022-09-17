mod error;
mod package;
mod tarball;
mod validate;

pub use error::Error;
pub use package::{
    AnyRef, Artifact, Definition, Namespace, ObjectKey, PackageKey,
    PackageMeta, PackageName, PackageReader, PackageSignature, PathRef,
    Pointer, Receipt, RegistryKind,
};
pub use validate::{confusable_skeleton, validate_id};

pub type Result<T> = std::result::Result<T, error::Error>;

/// Name of the header used for signatures.
pub const X_SIGNATURE: &str = "x-signature";

/// Well known message used for self-signing.
pub const WELL_KNOWN_MESSAGE: &[u8] = b".ipfs-registry";
