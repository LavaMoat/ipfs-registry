use flate2::read::GzDecoder;
use std::{io::prelude::*, path::PathBuf};
use tar::Archive;

use crate::{Descriptor, Error, Result};

const NPM: &str = "package/package.json";

/// Decompress a gzip buffer.
pub(crate) fn decompress(buffer: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(buffer);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

/// Read a package descriptor from an NPM compatible tarball.
pub(crate) fn read_npm_package(buffer: &[u8]) -> Result<Descriptor> {
    let package_path = PathBuf::from(NPM);
    let buffer = find_tar_entry(package_path, buffer)?;
    let descriptor: Descriptor = serde_json::from_slice(buffer)?;
    return Ok(descriptor);
}

/// Find the file data for a specific entry in a tarball.
fn find_tar_entry(
    package_path: PathBuf, buffer: &[u8]) -> Result<&[u8]> {
    let mut archive = Archive::new(buffer);
    for entry in archive.entries()? {
        let entry = entry?;
        let path = entry.path()?;
        if path.as_ref() == package_path.as_path() {
            let start_byte = entry.raw_file_position() as usize;
            let entry_size = entry.header().entry_size()? as usize;
            let end_byte = start_byte + entry_size;
            let file_bytes = &buffer[start_byte..end_byte];
            return Ok(file_bytes)
        }
    }
    Err(Error::NoPackage(package_path))
}
