use serde::{Serialize, Deserialize};
use reqwest::Client;
use k256::ecdsa::{recoverable, signature::Signer};
use bytes::Bytes;
use url::Url;

use ipfs_registry_core::X_SIGNATURE;

use crate::{
    Result,
    config::WebHookConfig,
};

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
pub async fn execute_webhooks<T: Serialize>(hooks: WebHookConfig, packet: WebHookPacket<T>) {
    match execute(hooks, packet).await {
        Ok(_) => {}
        Err(e) => tracing::error!("{}", e),
    }
}

/// Execute the configured webhooks.
async fn execute<T: Serialize>(hooks: WebHookConfig, packet: WebHookPacket<T>) -> Result<()> {
    let signing_key = hooks.signing_key.unwrap();
    let body = Bytes::from(serde_json::to_vec(&packet)?);
    let signature: recoverable::Signature =
        signing_key.sign(&body);
    let sign_bytes = &signature;
    for url in hooks.endpoints {
        request(url, body.clone(), sign_bytes).await?;
    }
    Ok(())
}

async fn request(url: Url, body: Bytes, sign_bytes: &recoverable::Signature) -> Result<bool> {
    let client = Client::new();
    let response = client
        .post(url)
        .body(body.clone())
        .header(X_SIGNATURE, base64::encode(sign_bytes))
        .send()
        .await?;
    Ok(response.status().is_success())
}
