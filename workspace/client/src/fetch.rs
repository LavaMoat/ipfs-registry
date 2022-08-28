use std::path::PathBuf;
use std::borrow::BorrowMut;

use semver::Version;
use url::Url;
use web3_address::ethereum::Address;
use reqwest::Client;
use tokio::io::AsyncWriteExt;
use human_bytes::human_bytes;

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

    let url = server.join(
        &format!("api/package/{}/{}/{}", address, name, version))?;

    let client = Client::new();
    let mut response = client.get(url).send().await?;

    response
        .status()
        .is_success()
        .then_some(())
        .ok_or(Error::ResponseCode(response.status().into()))?;

    let mut fd = tokio::fs::File::create(&file).await?;
    while let Some(mut item) = response.chunk().await? {
        fd.write_all_buf(item.borrow_mut()).await?;
    }

    fd.flush().await?;

    let size = file.metadata()?.len();
    tracing::info!(file = ?file, size = ?human_bytes(size as f64));

    Ok(())
}
