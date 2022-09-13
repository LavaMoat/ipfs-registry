use std::{borrow::BorrowMut, path::PathBuf};

use k256::ecdsa::{recoverable, signature::Signer, SigningKey};
use mime::Mime;
use reqwest::Client;

use tokio::io::AsyncWriteExt;
use url::Url;

use ipfs_registry_core::{
    Namespace, PackageKey, Receipt, WELL_KNOWN_MESSAGE, X_SIGNATURE,
};

use ipfs_registry_database::{NamespaceRecord, PublisherRecord};

use crate::{Error, Result};

/// Package registry client implementation.
pub struct RegistryClient;

impl RegistryClient {
    /// Create a publisher address.
    pub async fn signup(
        server: Url,
        signing_key: SigningKey,
    ) -> Result<PublisherRecord> {
        let signature: recoverable::Signature =
            signing_key.sign(WELL_KNOWN_MESSAGE);
        let sign_bytes = &signature;

        let client = Client::new();
        let url = server.join("api/publisher")?;

        let response = client
            .post(url)
            .header(X_SIGNATURE, base64::encode(sign_bytes))
            .send()
            .await?;

        response
            .status()
            .is_success()
            .then_some(())
            .ok_or_else(|| Error::ResponseCode(response.status().into()))?;

        let record: PublisherRecord = response.json().await?;
        Ok(record)
    }

    /// Register a namespace.
    pub async fn register(
        server: Url,
        signing_key: SigningKey,
        namespace: Namespace,
    ) -> Result<NamespaceRecord> {
        let signature: recoverable::Signature =
            signing_key.sign(namespace.as_bytes());
        let sign_bytes = &signature;

        let client = Client::new();
        let url = server.join("api/namespace")?
            .join(namespace.as_str())?;

        let response = client
            .post(url)
            .header(X_SIGNATURE, base64::encode(sign_bytes))
            .send()
            .await?;

        response
            .status()
            .is_success()
            .then_some(())
            .ok_or_else(|| Error::ResponseCode(response.status().into()))?;

        let record: NamespaceRecord = response.json().await?;
        Ok(record)
    }

    /// Download a package and write it to file.
    pub async fn fetch_file(
        server: Url,
        key: PackageKey,
        file: PathBuf,
    ) -> Result<PathBuf> {
        if file.exists() {
            return Err(Error::FileExists(file));
        }

        let url = server.join("api/package")?;

        let client = Client::new();
        let request = client.get(url).query(&[("id", key.to_string())]);

        let mut response = request.send().await?;

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

    /// Publish a package file with the given signing key.
    pub async fn publish_file(
        server: Url,
        namespace: Namespace,
        mime: Mime,
        signing_key: SigningKey,
        file: PathBuf,
    ) -> Result<Receipt> {
        if !file.is_file() {
            return Err(Error::NotFile(file));
        }

        let body = std::fs::read(file)?;
        let signature: recoverable::Signature = signing_key.sign(&body);
        let sign_bytes = &signature;

        let client = Client::new();
        let url = server.join("api/package")?.join(namespace.as_str())?;

        let response = client
            .post(url)
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

        let doc: Receipt = response.json().await?;
        Ok(doc)
    }
}
