use std::{io::prelude::*, path::PathBuf};

use flate2::read::GzDecoder;
use serde::Deserialize;
use tar::Archive;

use crate::{Error, PackageMeta, Result};

const NPM: &str = "package/package.json";
const CARGO: &str = "Cargo.toml";

#[derive(Deserialize)]
struct CargoPackage {
    package: PackageMeta,
}

/// Decompress a gzip buffer.
pub(crate) fn decompress(buffer: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(buffer);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

pub(crate) fn remove_npm_scope(
    mut descriptor: PackageMeta,
) -> Result<PackageMeta> {
    let needle = "/";
    if let Some(index) = descriptor.name.as_str().rfind(needle) {
        let name = &descriptor.name.as_str()[index + needle.len()..];
        descriptor.name = name.parse()?;
    }
    Ok(descriptor)
}

/// Read a package descriptor from an NPM compatible tarball.
pub(crate) fn read_npm_package(
    buffer: &[u8],
) -> Result<(PackageMeta, &[u8])> {
    let package_path = PathBuf::from(NPM);
    let buffer = find_tar_entry(package_path, buffer, true)?;
    let descriptor: PackageMeta = serde_json::from_slice(buffer)?;
    let descriptor = remove_npm_scope(descriptor)?;
    Ok((descriptor, buffer))
}

/// Read a package descriptor from a Cargo compatible tarball.
pub(crate) fn read_cargo_package(
    buffer: &[u8],
) -> Result<(PackageMeta, &[u8])> {
    let package_path = PathBuf::from(CARGO);
    let buffer = find_tar_entry(package_path, buffer, false)?;
    let descriptor: CargoPackage = toml::from_slice(buffer)?;
    Ok((descriptor.package, buffer))
}

/// Find the file data for a specific entry in a tarball.
fn find_tar_entry(
    package_path: PathBuf,
    buffer: &[u8],
    exact: bool,
) -> Result<&[u8]> {
    let mut archive = Archive::new(buffer);
    for entry in archive.entries()? {
        let entry = entry?;
        let path = entry.path()?;

        let matched = if exact {
            path.as_ref() == package_path.as_path()
        } else {
            path.as_ref().ends_with(package_path.as_path())
        };

        if matched {
            let start_byte = entry.raw_file_position() as usize;
            let entry_size = entry.header().entry_size()? as usize;
            let end_byte = start_byte + entry_size;
            let file_bytes = &buffer[start_byte..end_byte];
            return Ok(file_bytes);
        }
    }
    Err(Error::NoPackage(package_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PackageName;
    use anyhow::Result;
    use semver::Version;
    use std::path::PathBuf;

    #[test]
    fn scope_remove() -> Result<()> {
        let descriptor = PackageMeta {
            name: PackageName::new_unchecked("@mock-scope/mock-package"),
            version: Version::new(1, 0, 0),
        };

        let descriptor = remove_npm_scope(descriptor)?;
        assert_eq!(
            PackageName::new_unchecked("mock-package"),
            descriptor.name
        );
        Ok(())
    }

    #[test]
    fn decompress_tarball() -> Result<()> {
        let file = PathBuf::from("../../fixtures/mock-package-1.0.0.tgz");
        let contents = std::fs::read(&file)?;
        let decompressed = decompress(&contents)?;
        let (descriptor, _) = read_npm_package(&decompressed)?;
        assert_eq!(1u64, descriptor.version.major);
        assert_eq!(
            PackageName::new_unchecked("mock-package"),
            descriptor.name
        );
        Ok(())
    }
}
