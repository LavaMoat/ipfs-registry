use serde::Deserialize;
use semver::Version;

/// Package descriptor.
#[derive(Debug, Deserialize)]
pub struct Descriptor {
    pub name: String,
    pub version: Version,
}
