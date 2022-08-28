mod error;
mod package;
mod tarball;

pub use error::Error;
pub use package::{PackageReader, Descriptor, Definition, RegistryKind};

pub type Result<T> = std::result::Result<T, error::Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::path::PathBuf;

    #[test]
    fn decompress_tarball() -> Result<()> {
        let file = PathBuf::from("../../fixtures/mock-package-1.0.0.tgz");
        let contents = std::fs::read(&file)?;
        let decompressed = decompress(&contents)?;
        let descriptor = read_npm_package(&decompressed)?;
        assert_eq!("mock-package", &descriptor.name);
        assert_eq!(1u64, descriptor.version.major);
        Ok(())
    }
}
