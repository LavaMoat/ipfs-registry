use k256::ecdsa::{recoverable, signature::Signer, SigningKey};
use mime::Mime;
use reqwest::Client;
use secrecy::ExposeSecret;
use std::path::PathBuf;
use url::Url;
use web3_keystore::{decrypt, KeyStore};

use ipfs_registry_core::{Document, X_SIGNATURE};

use crate::{input::read_password, Error, Result};

/// Publish a package.
pub async fn publish(
    server: Url,
    mime: Mime,
    key: PathBuf,
    file: PathBuf,
) -> Result<Document> {
    if !file.is_file() {
        return Err(Error::NotFile(file));
    }

    let buffer = std::fs::read(key)?;
    let keystore: KeyStore = serde_json::from_slice(&buffer)?;

    let password = read_password(Some("Keystore passphrase: "))?;

    let key = decrypt(&keystore, password.expose_secret())?;
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
        .ok_or_else(|| Error::ResponseCode(response.status().into()))?;

    let doc: Document = response.json().await?;
    Ok(doc)
}
