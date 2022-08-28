use mime::Mime;
use semver::Version;
use std::path::PathBuf;
use url::Url;
use web3_address::ethereum::Address;

use crate::{Error, Result};

/// Download a package and write it to file.
pub async fn fetch(
    server: Url,
    address: Address,
    name: String,
    version: Version,
    file: PathBuf,
) -> Result<()> {
    if file.exists() {
        return Err(Error::FileExists(file));
    }

    todo!()
}
