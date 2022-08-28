use mime::Mime;
use reqwest::{Body, Client};
use std::path::PathBuf;
use url::Url;
use k256::ecdsa::{SigningKey, signature::Signer, recoverable};
use web3_keystore::{KeyStore, decrypt};

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

    let buffer = std::fs::read(key)?;
    let keystore: KeyStore = serde_json::from_slice(&buffer)?;

    // TODO: get password from stdin
    let password = String::from("");

    let key = decrypt(&keystore, &password)?;
    let signing_key = SigningKey::from_bytes(&key)?;

    let body = std::fs::read(file)?;
    let signature: recoverable::Signature = signing_key.sign(&body);
    let sign_bytes = &signature;

    let client = Client::new();
    let url = server.join("api/package")?;

    let response = client
        .put(url)
        .header(X_SIGNATURE, base64::encode(sign_bytes))
        .header("content-type", mime.to_string())
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
