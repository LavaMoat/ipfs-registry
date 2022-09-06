use std::borrow::BorrowMut;
use std::path::PathBuf;

use reqwest::Client;
use semver::Version;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::{Error, Result};

/// Download a package and write it to file.
pub async fn fetch(
    server: Url,
    organization: String,
    name: String,
    version: Version,
    file: PathBuf,
) -> Result<PathBuf> {
    if file.exists() {
        return Err(Error::FileExists(file));
    }

    let url = server.join(&format!(
        "api/package/{}/{}/{}",
        organization, name, version
    ))?;

    let client = Client::new();
    let mut response = client.get(url).send().await?;

    response
        .status()
        .is_success()
        .then_some(())
        .ok_or_else(|| Error::ResponseCode(response.status().into()))?;

    let mut fd = tokio::fs::File::create(&file).await?;
    while let Some(mut item) = response.chunk().await? {
        fd.write_all_buf(item.borrow_mut()).await?;
    }

    fd.flush().await?;

    Ok(file)
}
