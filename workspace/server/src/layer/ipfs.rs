//! IPFS backed storage layer.
use async_trait::async_trait;
use axum::{body::Bytes, http::uri::Scheme};
use futures::TryStreamExt;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient, TryFromUri};
use std::io::Cursor;
use url::Url;

use ipfs_registry_core::{Artifact, ObjectKey};

use hyper::Uri;

use super::Layer;

use crate::{Error, Result};

/// Layer for IPFS backed storage.
pub struct IpfsLayer {
    client: IpfsClient,
}

impl IpfsLayer {
    /// Create a new IPFS storage layer.
    pub fn new(url: &Url) -> Result<Self> {
        let client = IpfsLayer::new_client(url)?;
        Ok(Self { client })
    }

    /// Create a new IPFS client from the configuration URL.
    fn new_client(url: &Url) -> Result<IpfsClient> {
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

        let uri: Uri = url.to_string().parse()?;

        // /ip4/127.0.0.1/tcp/5001

        //println!("Creating new IPFS layer with {}", uri);

        if Scheme::HTTPS == scheme {
            let client = IpfsClient::build_with_base_uri(uri);
            Ok(client)
        } else {
            Ok(IpfsClient::from_host_and_port(scheme, host, port)?)
        }
    }
}

#[async_trait]
impl Layer for IpfsLayer {
    async fn add_blob(
        &self,
        data: Bytes,
        _descriptor: &Artifact,
    ) -> Result<Vec<ObjectKey>> {
        let data = Cursor::new(data);
        let add_res = self.client.add(data).await?;
        self.client.pin_add(&add_res.hash, true).await?;
        Ok(vec![ObjectKey::Cid(add_res.hash.try_into()?)])
    }

    async fn get_blob(&self, id: &ObjectKey) -> Result<Vec<u8>> {
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
