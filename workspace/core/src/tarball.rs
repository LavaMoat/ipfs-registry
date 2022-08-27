use std::{io::prelude::*, path::PathBuf};
use flate2::read::GzDecoder;
use tar::Archive;

use crate::{Error, Result, Descriptor};

/// Decompress a gzip buffer.
pub fn decompress(buffer: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(buffer);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

/// Read a package descriptor from a tar file.
pub fn read_npm_package(buffer: &[u8]) -> Result<Descriptor> {
    let package_path = PathBuf::from("package/package.json");
    let mut archive = Archive::new(buffer);
    for entry in archive.entries()? {
        let entry = entry?;
        let path = entry.path()?;
        if path.as_ref() == package_path.as_path() {
            let start_byte = entry.raw_file_position() as usize;
            let entry_size = entry.header().entry_size()? as usize;
            let end_byte = start_byte + entry_size;
            let file_bytes = &buffer[start_byte..end_byte];
            let descriptor: Descriptor = 
                serde_json::from_slice(file_bytes)?;
            return Ok(descriptor)
        }
    }
    Err(Error::NoNpmPackage)
}
