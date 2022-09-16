use std::time::Duration;
use bytes::Bytes;
use k256::ecdsa::{recoverable, signature::Signer};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

use ipfs_registry_core::X_SIGNATURE;

use crate::{config::WebHookConfig, Result};

#[derive(Serialize, Deserialize)]
#[serde(untagged, rename_all = "lowercase")]
pub enum WebHookEvent {
    /// Event triggered when a package is fetched.
    Fetch,
    /// Event triggered when a package is published.
    Publish,
}

#[derive(Serialize, Deserialize)]
pub struct WebHookBody<T> {
    #[serde(flatten)]
    pub(crate) inner: T,
}

#[derive(Serialize, Deserialize)]
pub struct WebHookPacket<T> {
    pub event: WebHookEvent,
    pub body: WebHookBody<T>,
}

/// Execute the configured webhooks.
pub async fn execute_webhooks<T: Serialize>(
    hooks: WebHookConfig,
    packet: WebHookPacket<T>,
) {
    match execute(hooks, packet).await {
        Ok(_) => {}
        Err(e) => tracing::error!("{}", e),
    }
}

async fn execute<T: Serialize>(
    hooks: WebHookConfig,
    packet: WebHookPacket<T>,
) -> Result<()> {
    let signing_key = hooks.signing_key.unwrap();
    let body = Bytes::from(serde_json::to_vec(&packet)?);
    let signature: recoverable::Signature = signing_key.sign(&body);
    for url in hooks.endpoints {
        tokio::spawn(request_with_retry(
            hooks.retry_limit,
            hooks.backoff_seconds,
            url, body.clone(), signature.clone()));
    }
    Ok(())
}

async fn request_with_retry(
    retry_limit: u64,
    backoff_seconds: u64,
    url: Url,
    body: Bytes,
    signature: recoverable::Signature,
    ) -> Result<bool> {

    let backoff_millis = backoff_seconds * 1000;
    for index in 0..retry_limit {
        let success = request(url.clone(), body.clone(), signature).await?;
        if success {
            return Ok(true)
        }
        tokio::time::sleep(
            Duration::from_millis(backoff_millis * (index + 1))).await;
    }
    Ok(false)
}

async fn request(
    url: Url,
    body: Bytes,
    signature: recoverable::Signature,
) -> Result<bool> {
    let client = Client::new();
    let response = client
        .post(url)
        .body(body.clone())
        .header(X_SIGNATURE, base64::encode(&signature))
        .send()
        .await?;
    Ok(response.status().is_success())
}
