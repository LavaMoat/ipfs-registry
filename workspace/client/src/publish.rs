use mime::Mime;
use reqwest::{Body, Client};
use std::path::PathBuf;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use url::Url;

use ipfs_registry_core::X_SIGNATURE;

use crate::{Error, Result};

/// Publish a package.
pub async fn publish(
    server: Url,
    mime: Mime,
    key: PathBuf,
    file: PathBuf,
) -> Result<()> {
    if !file.is_file() {
        return Err(Error::NotFile(file));
    }

    let client = Client::new();
    let url = server.join("api/package")?;
    let body = file_to_body(File::from_std(std::fs::File::open(file)?));

    let response = client
        .put(url)
        .header(X_SIGNATURE, "")
        .body(body)
        .send()
        .await?;

    response
        .status()
        .is_success()
        .then_some(())
        .ok_or(Error::ResponseCode(response.status().into()))?;

    Ok(())
}

fn file_to_body(file: File) -> Body {
    let stream = FramedRead::new(file, BytesCodec::new());
    let body = Body::wrap_stream(stream);
    body
}
