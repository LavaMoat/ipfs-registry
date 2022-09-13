//! IPFS backed storage layer.
use async_trait::async_trait;
use axum::{body::Bytes, http::uri::Scheme};
use futures::TryStreamExt;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient, TryFromUri};
use std::io::Cursor;
use url::Url;

use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector;

use ipfs_registry_core::{Artifact, ObjectKey};

use super::Layer;

use crate::{Error, Result};

/// Layer for IPFS backed storage.
pub struct IpfsLayer {
    client: IpfsClient<HttpsConnector<HttpConnector>>,
}

impl IpfsLayer {
    /// Create a new IPFS storage layer.
    pub fn new(url: &Url) -> Result<Self> {
        let client = IpfsLayer::new_client(url)?;
        Ok(Self { client })
    }

    /// Create a new IPFS client from the configuration URL.
    fn new_client(
        url: &Url,
    ) -> Result<IpfsClient<HttpsConnector<HttpConnector>>> {
        let host = url
            .host_str()
            .ok_or_else(|| Error::InvalidHost(url.clone()))?;
        let port = url
            .port_or_known_default()
            .ok_or_else(|| Error::InvalidPort(url.clone()))?;
        let scheme = if url.scheme() == "http" {
            Scheme::HTTP
        } else if url.scheme() == "https" {
            Scheme::HTTPS
        } else {
            return Err(Error::InvalidScheme(url.scheme().to_owned()));
        };

        tracing::info!(url = %url);

        Ok(
            IpfsClient::<HttpsConnector<HttpConnector>>::from_host_and_port(
                scheme, host, port,
            )?,
        )
    }
}

#[async_trait]
impl Layer for IpfsLayer {
    fn supports_content_id(&self) -> bool {
        true
    }

    async fn add_artifact(
        &self,
        data: Bytes,
        _descriptor: &Artifact,
    ) -> Result<ObjectKey> {
        let data = Cursor::new(data);
        let add_res = self.client.add(data).await?;
        self.client.pin_add(&add_res.hash, true).await?;
        Ok(ObjectKey::Cid(add_res.hash.try_into()?))
    }

    async fn get_artifact(&self, id: &ObjectKey) -> Result<Vec<u8>> {
        let id = id.to_string();
        let res = self
            .client
            .cat(&id)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
            .await?;
        Ok(res)
    }
}
