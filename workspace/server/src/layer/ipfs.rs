//! IPFS backed storage layer.
use async_trait::async_trait;
use axum::{body::Bytes, http::uri::Scheme};
use futures::TryStreamExt;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient, TryFromUri};
use std::io::Cursor;
use url::Url;
use web3_address::ethereum::Address;

use ipfs_registry_core::{
    Definition, Descriptor, NamespacedDescriptor, PackagePointer, Receipt,
    RegistryKind,
};

use serde_json::Value;

use super::Layer;

use super::{NAME, ROOT};
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
        Ok(IpfsClient::from_host_and_port(scheme, host, port)?)
    }
}

#[async_trait]
impl Layer for IpfsLayer {
    async fn add_blob(
        &self,
        data: Bytes,
        _descriptor: &NamespacedDescriptor,
    ) -> Result<String> {
        let data = Cursor::new(data);
        let add_res = self.client.add(data).await?;
        self.client.pin_add(&add_res.hash, true).await?;
        Ok(add_res.hash)
    }

    async fn get_blob(&self, id: &str) -> Result<Vec<u8>> {
        let res = self
            .client
            .cat(id)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
            .await?;
        Ok(res)
    }

    async fn add_pointer(
        &self,
        signature: String,
        address: &Address,
        descriptor: NamespacedDescriptor,
        archive_id: String,
        package: Value,
    ) -> Result<Receipt> {
        let dir = format!(
            "/{}/{}/{}/{}/{}",
            ROOT,
            &descriptor.kind,
            &descriptor.namespace,
            &descriptor.package.name,
            &descriptor.package.version
        );

        self.client.files_mkdir(&dir, true).await?;

        let definition = Definition {
            descriptor,
            archive: archive_id,
            signature,
        };

        let doc = PackagePointer {
            definition: definition.clone(),
            package,
        };
        let data = serde_json::to_vec_pretty(&doc)?;
        let path = format!("{}/{}", dir, NAME);

        let data = Cursor::new(data);
        self.client.files_write(&path, true, true, data).await?;
        self.client.files_flush(Some(&path)).await?;

        let stat = self.client.files_stat(&path).await?;
        self.client.pin_add(&stat.hash, true).await?;

        let receipt = Receipt {
            pointer: stat.hash,
            definition,
        };

        Ok(receipt)
    }

    async fn get_pointer(
        &self,
        descriptor: &NamespacedDescriptor,
    ) -> Result<Option<PackagePointer>> {
        let path = format!(
            "/{}/{}/{}/{}/{}/{}",
            ROOT,
            &descriptor.kind,
            &descriptor.namespace,
            &descriptor.package.name,
            &descriptor.package.version,
            NAME
        );

        let result = if let Ok(res) = self
            .client
            .files_read(&path)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
            .await
        {
            let doc: PackagePointer = serde_json::from_slice(&res)?;
            Some(doc)
        } else {
            None
        };

        Ok(result)
    }
}
