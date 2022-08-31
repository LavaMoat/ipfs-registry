//! S3 backed storage layer.
use async_trait::async_trait;
use axum::{body::Bytes, http::uri::Scheme};
use futures::TryStreamExt;
use semver::Version;
use web3_address::ethereum::Address;

use serde_json::Value;

use super::Layer;

use rusoto_core::credential;
use rusoto_core::request::HttpClient;
//use rusoto_core::ByteStream;
use rusoto_core::Region;
use rusoto_s3::S3Client;

use ipfs_registry_core::{
    Definition, Descriptor, PackagePointer, Receipt, RegistryKind,
};

use crate::{Error, Result};

const ROOT: &str = "ipfs-registry";
const NAME: &str = "meta.json";

/// Layer for S3 backed storage.
pub struct S3Layer {
    client: S3Client,
}

impl S3Layer {
    /// Create a new S3 storage layer.
    pub fn new(profile: &str, region: &Region) -> Result<Self> {
        let client = S3Layer::new_client(profile, region)?;
        Ok(Self { client })
    }


    fn new_client(profile: &str, region: &Region) -> Result<S3Client> {
        let mut provider = credential::ProfileProvider::new()?;
        provider.set_profile(profile);
        let dispatcher = HttpClient::new()?;
        let client = S3Client::new_with(dispatcher, provider, region.clone());
        Ok(client)
    }
}

#[async_trait]
impl Layer for S3Layer {
    async fn add_blob(&self, data: Bytes) -> Result<String> {
        todo!()
    }

    async fn get_blob(&self, id: &str) -> Result<Vec<u8>> {
        todo!()
    }

    async fn add_pointer(
        &self,
        kind: RegistryKind,
        signature: String,
        address: &Address,
        descriptor: Descriptor,
        archive_id: String,
        package: Value,
    ) -> Result<Receipt> {
        todo!()
    }

    async fn get_pointer(
        &self,
        kind: RegistryKind,
        address: &Address,
        name: &str,
        version: &Version,
    ) -> Result<Option<PackagePointer>> {
        todo!()
    }
}
