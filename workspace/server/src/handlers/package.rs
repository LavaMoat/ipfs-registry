use axum::{
    Json,
    body::Bytes,
    extract::{Extension, Path, TypedHeader},
    headers::ContentType,
    http::{uri::Scheme, HeaderMap, StatusCode},
};

//use axum_macros::debug_handler;

use futures::TryStreamExt;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient, TryFromUri};
use k256::ecdsa::recoverable;
use semver::Version;
use std::{io::Cursor, sync::Arc};
use tokio::sync::RwLock;
use url::Url;
use web3_address::ethereum::Address;

use ipfs_registry_core::{Definition, Descriptor, PackageReader, RegistryKind};

use crate::{headers::Signature, Error, Result, State};

const ROOT: &str = "ipfs-registry";
const NAME: &str = "meta.json";

/// Verify a signature against a message and return the address.
fn verify_signature(signature: [u8; 65], message: &[u8]) -> Result<Address> {
    let recoverable: recoverable::Signature =
        signature.as_slice().try_into()?;
    let public_key = recoverable.recover_verifying_key(message)?;
    let public_key: [u8; 33] = public_key.to_bytes().as_slice().try_into()?;
    let address: Address = (&public_key).try_into()?;
    Ok(address)
}

struct Ipfs;

impl Ipfs {
    /// Create a new IPFS client from the configuration URL.
    fn new_client(url: &Url) -> Result<IpfsClient> {
        let host = url.host_str().ok_or(Error::InvalidHost(url.clone()))?;
        let port = url
            .port_or_known_default()
            .ok_or(Error::InvalidPort(url.clone()))?;

        let scheme = if url.scheme() == "http" {
            Scheme::HTTP
        } else if url.scheme() == "https" {
            Scheme::HTTPS
        } else {
            return Err(Error::InvalidScheme(url.scheme().to_owned()));
        };

        Ok(IpfsClient::from_host_and_port(scheme, host, port)?)
    }

    /// Add a blob to IPFS.
    async fn add(url: &Url, data: Bytes) -> Result<String> {
        let client = Ipfs::new_client(url)?;
        let data = Cursor::new(data);
        let add_res = client.add(data).await?;
        let _pin_res = client.pin_add(&add_res.hash, true).await?;
        Ok(add_res.hash)
    }

    /// Get a blob from IPFS.
    async fn get(url: &Url, cid: &str) -> Result<Vec<u8>> {
        let client = Ipfs::new_client(url)?;
        let res = client
            .cat(cid)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
            .await?;
        Ok(res)
    }
}

/// Manage access to the package index.
struct Index;

impl Index {
    /// Add a package to the index.
    async fn add_package(
        url: &Url,
        kind: RegistryKind,
        address: &Address,
        descriptor: Descriptor,
        cid: String,
    ) -> Result<Definition> {
        // TODO: unpin an existing version?

        let dir = format!(
            "/{}/{}/{}/{}/{}",
            ROOT,
            kind, address, descriptor.name, descriptor.version
        );

        let client = Ipfs::new_client(&url)?;

        client.files_mkdir(&dir, true).await?;

        let definition = Definition { descriptor, cid };

        let data = serde_json::to_vec(&definition)?;
        let path = format!("{}/{}", dir, NAME);

        let data = Cursor::new(data);
        client.files_write(&path, true, true, data).await?;
        client.files_flush(Some(&path)).await?;

        // TODO: pin the new version

        Ok(definition)
    }

    /// Get a package from the index.
    async fn get_package(
        url: &Url,
        kind: RegistryKind,
        address: &Address,
        name: &str,
        version: &Version,
    ) -> Result<Option<Definition>> {
        let client = Ipfs::new_client(&url)?;

        let path = format!(
            "/{}/{}/{}/{}/{}/{}",
            ROOT, kind, address, name, version, NAME
        );

        let result = if let Ok(res) = client
            .files_read(&path)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
            .await
        {
            let doc: Definition = serde_json::from_slice(&res)?;
            Some(doc)
        } else {
            None
        };

        Ok(result)
    }
}

pub(crate) struct PackageHandler;
impl PackageHandler {
    /// Get a package.
    pub(crate) async fn get(
        Extension(state): Extension<Arc<RwLock<State>>>,
        Path((address, name, version)): Path<(Address, String, Version)>,
    ) -> std::result::Result<(HeaderMap, Bytes), StatusCode> {
        let reader = state.read().await;
        let url = reader.config.ipfs.url.clone();
        let mime_type = reader.config.registry.mime.clone();
        let kind = reader.config.registry.kind.clone();
        drop(reader);

        //let address = String::from("mock-address");

        tracing::debug!(
            address = %address,
            name = %name,
            version = ?version);

        // Get the package meta data
        let meta = Index::get_package(&url, kind, &address, &name, &version)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        tracing::debug!(meta = ?meta);

        if let Some(doc) = meta {
            let body = Ipfs::get(&url, &doc.cid)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let mut headers = HeaderMap::new();
            headers.insert("content-type", mime_type.parse().unwrap());

            Ok((headers, Bytes::from(body)))
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    }

    /// Create a new package.
    pub(crate) async fn put(
        Extension(state): Extension<Arc<RwLock<State>>>,
        TypedHeader(mime): TypedHeader<ContentType>,
        TypedHeader(signature): TypedHeader<Signature>,
        body: Bytes,
    ) -> std::result::Result<Json<Definition>, StatusCode> {
        // Verify the signature header against the payload bytes
        let address = verify_signature(signature.into(), &body)
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let reader = state.read().await;
        let url = reader.config.ipfs.url.clone();
        let mime_type = reader.config.registry.mime.clone();
        let kind = reader.config.registry.kind.clone();
        drop(reader);

        tracing::debug!(mime = ?mime_type);

        // TODO: ensure approval signatures

        let gzip: mime::Mime = mime_type
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let gzip_ct = ContentType::from(gzip);

        if mime == gzip_ct {
            let descriptor = PackageReader::read(kind, &body)
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            // Check the package version does not already exist
            let meta = Index::get_package(
                &url,
                kind,
                &address,
                &descriptor.name,
                &descriptor.version,
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            if meta.is_some() {
                return Err(StatusCode::CONFLICT);
            }

            // TODO: store in the index

            let cid = Ipfs::add(&url, body)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            tracing::debug!(cid = %cid, "added package");

            // Store the package meta data
            let definition = Index::add_package(&url, kind, &address, descriptor, cid)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            Ok(Json(definition))
        } else {
            Err(StatusCode::BAD_REQUEST)
        }
    }
}
