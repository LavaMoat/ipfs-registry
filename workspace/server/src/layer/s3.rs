//! S3 backed storage layer.
use async_trait::async_trait;
use axum::body::Bytes;
use futures::TryStreamExt;
use serde_json::Value;
use tokio_util::codec;
use web3_address::ethereum::Address;

use rusoto_core::{
    credential, request::HttpClient, ByteStream, Region, RusotoError,
};
use rusoto_s3::{
    GetObjectError, GetObjectRequest, PutObjectOutput, PutObjectRequest,
    S3Client, S3,
};

use ipfs_registry_core::{Artifact, Definition, ObjectKey, Pointer};

use super::Layer;
use super::{BLOB, NAME, ROOT};
use crate::{Error, Result};

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

    fn get_blob_key(&self, artifact: &Artifact) -> String {
        format!(
            "{}/{}/{}/{}/{}/{}",
            ROOT,
            &artifact.kind,
            &artifact.namespace,
            &artifact.package.name,
            &artifact.package.version,
            BLOB,
        )
    }

    fn get_pointer_key(&self, artifact: &Artifact) -> String {
        format!(
            "{}/{}/{}/{}/{}/{}",
            ROOT,
            &artifact.kind,
            &artifact.namespace,
            &artifact.package.name,
            &artifact.package.version,
            NAME
        )
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

    async fn get_object(&self, key: String) -> Result<Option<Vec<u8>>> {
        let req = GetObjectRequest {
            bucket: self.bucket.clone(),
            key,
            ..Default::default()
        };

        let result = self.client.get_object(req).await;

        if let Err(RusotoError::<GetObjectError>::Service(
            GetObjectError::NoSuchKey(_),
        )) = &result
        {
            return Ok(None);
        }

        if let Some(body) = result?.body {
            let content = codec::FramedRead::new(
                body.into_async_read(),
                codec::BytesCodec::new(),
            );

            let mut buf: Vec<u8> = Vec::new();
            content
                .try_for_each(|bytes| {
                    buf.extend(&bytes);
                    futures::future::ok(())
                })
                .await?;

            Ok(Some(buf))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl Layer for S3Layer {
    async fn add_blob(
        &self,
        data: Bytes,
        artifact: &Artifact,
    ) -> Result<Vec<ObjectKey>> {
        let key = self.get_blob_key(artifact);
        self.put_object(key.clone(), data).await?;
        Ok(vec![ObjectKey::Key(key)])
    }

    async fn get_blob(&self, id: &ObjectKey) -> Result<Vec<u8>> {
        if let ObjectKey::Key(key) = id {
            let result = self
                .get_object(key.to_owned())
                .await?
                .ok_or_else(|| Error::ObjectMissing(key.to_string()))?;
            Ok(result)
        } else {
            Err(Error::BadObjectKey)
        }
    }

    async fn add_pointer(
        &self,
        signature: String,
        _address: &Address,
        artifact: Artifact,
        mut objects: Vec<ObjectKey>,
        package: Value,
    ) -> Result<Vec<ObjectKey>> {
        let key = self.get_pointer_key(&artifact);

        let object = objects.remove(0);

        let definition = Definition {
            artifact,
            object,
            signature,
        };

        let doc = Pointer {
            definition: definition.clone(),
            package,
        };

        let data = serde_json::to_vec_pretty(&doc)?;
        self.put_object(key.clone(), Bytes::from(data)).await?;

        Ok(vec![ObjectKey::Key(key)])
    }

    async fn get_pointer(
        &self,
        artifact: &Artifact,
    ) -> Result<Option<Pointer>> {
        let key = self.get_pointer_key(artifact);
        let result = if let Some(res) = self.get_object(key).await? {
            let doc: Pointer = serde_json::from_slice(&res)?;
            Some(doc)
        } else {
            None
        };
        Ok(result)
    }
}
