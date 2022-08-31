//! S3 backed storage layer.
use async_trait::async_trait;
use axum::body::Bytes;
use web3_address::ethereum::Address;

use serde_json::Value;

use super::Layer;

use rusoto_core::{credential, request::HttpClient, ByteStream, Region};
use rusoto_s3::{PutObjectOutput, PutObjectRequest, S3Client, S3};

use ipfs_registry_core::{
    Definition, NamespacedDescriptor, PackagePointer, Receipt,
};

use super::{NAME, ROOT, BLOB};
use crate::Result;

/// Layer for S3 backed storage.
pub struct S3Layer {
    client: S3Client,
    bucket: String,
    content_type: String,
}

impl S3Layer {
    /// Create a new S3 storage layer.
    pub fn new(
        profile: String,
        region: String,
        bucket: String,
        content_type: String,
    ) -> Result<Self> {
        let region: Region = region.parse()?;
        let client = S3Layer::new_client(&profile, &region)?;
        Ok(Self {
            client,
            bucket,
            content_type,
        })
    }

    fn new_client(profile: &str, region: &Region) -> Result<S3Client> {
        let mut provider = credential::ProfileProvider::new()?;
        provider.set_profile(profile);
        let dispatcher = HttpClient::new()?;
        let client = S3Client::new_with(dispatcher, provider, region.clone());
        Ok(client)
    }

    async fn put_object(
        &self,
        key: String,
        body: Bytes,
    ) -> Result<PutObjectOutput> {
        let size = body.len();
        let stream = futures::stream::once(futures::future::ok(body));
        let body = ByteStream::new_with_size(stream, size);

        let req = PutObjectRequest {
            bucket: self.bucket.clone(),
            key,
            content_type: Some(self.content_type.clone()),
            body: Some(body),
            ..Default::default()
        };

        Ok(self.client.put_object(req).await?)
    }
}

#[async_trait]
impl Layer for S3Layer {
    async fn add_blob(
        &self,
        data: Bytes,
        descriptor: &NamespacedDescriptor,
    ) -> Result<String> {
        let key = format!(
            "{}/{}/{}/{}/{}/{}",
            ROOT,
            &descriptor.kind,
            &descriptor.namespace,
            &descriptor.package.name,
            &descriptor.package.version,
            BLOB,
        );
        self.put_object(key.clone(), data).await?;
        Ok(key)
    }

    async fn get_blob(&self, _id: &str) -> Result<Vec<u8>> {
        todo!()
    }

    async fn add_pointer(
        &self,
        signature: String,
        _address: &Address,
        descriptor: NamespacedDescriptor,
        archive_id: String,
        package: Value,
    ) -> Result<Receipt> {
        let key = format!(
            "{}/{}/{}/{}/{}/{}",
            ROOT,
            &descriptor.kind,
            &descriptor.namespace,
            &descriptor.package.name,
            &descriptor.package.version,
            NAME
        );

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
        self.put_object(key.clone(), Bytes::from(data)).await?;

        let receipt = Receipt {
            pointer: key,
            definition,
        };

        Ok(receipt)
    }

    async fn get_pointer(
        &self,
        _descriptor: &NamespacedDescriptor,
    ) -> Result<Option<PackagePointer>> {
        todo!()
    }
}
