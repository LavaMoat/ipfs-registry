use flate2::read::GzDecoder;
use std::{io::prelude::*, path::PathBuf};
use tar::Archive;

use crate::{Error, PackageMeta, Result};

const NPM: &str = "package/package.json";

/// Decompress a gzip buffer.
pub(crate) fn decompress(buffer: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(buffer);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

pub(crate) fn remove_npm_scope(mut descriptor: PackageMeta) -> PackageMeta {
    let needle = "/";
    if let Some(index) = descriptor.name.rfind(needle) {
        let name = &descriptor.name[index + needle.len()..];
        descriptor.name = name.to_owned();
    }
    descriptor
}

/// Read a package descriptor from an NPM compatible tarball.
pub(crate) fn read_npm_package(
    buffer: &[u8],
) -> Result<(PackageMeta, &[u8])> {
    let package_path = PathBuf::from(NPM);
    let buffer = find_tar_entry(package_path, buffer)?;
    let descriptor: PackageMeta = serde_json::from_slice(buffer)?;
    let descriptor = remove_npm_scope(descriptor);
    Ok((descriptor, buffer))
}

/// Find the file data for a specific entry in a tarball.
fn find_tar_entry(package_path: PathBuf, buffer: &[u8]) -> Result<&[u8]> {
    let mut archive = Archive::new(buffer);
    for entry in archive.entries()? {
        let entry = entry?;
        let path = entry.path()?;
        if path.as_ref() == package_path.as_path() {
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
    use anyhow::Result;
    use semver::Version;
    use std::path::PathBuf;

    #[test]
    fn scope_remove() -> Result<()> {
        let descriptor = PackageMeta {
            name: "@mock-scope/mock-package".to_owned(),
            version: Version::new(1, 0, 0),
        };

        let descriptor = remove_npm_scope(descriptor);
        assert_eq!("mock-package", &descriptor.name);
        Ok(())
    }

    #[test]
    fn decompress_tarball() -> Result<()> {
        let file = PathBuf::from("../../fixtures/mock-package-1.0.0.tgz");
        let contents = std::fs::read(&file)?;
        let decompressed = decompress(&contents)?;
        let (descriptor, _) = read_npm_package(&decompressed)?;
        assert_eq!("mock-package", &descriptor.name);
        assert_eq!(1u64, descriptor.version.major);
        Ok(())
    }
}
