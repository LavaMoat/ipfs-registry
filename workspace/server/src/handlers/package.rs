use axum::{
    body::Bytes,
    extract::{Extension, Path, TypedHeader},
    headers::ContentType,
    http::{HeaderMap, StatusCode},
    Json,
};

//use axum_macros::debug_handler;

use k256::ecdsa::recoverable;
use semver::Version;

use web3_address::ethereum::Address;

use ipfs_registry_core::{PackageReader, Receipt};

use crate::{headers::Signature, server::ServerState, Result, layer::Layer};

/// Verify a signature against a message and return the address.
fn verify_signature(signature: [u8; 65], message: &[u8]) -> Result<Address> {
    let recoverable: recoverable::Signature =
        signature.as_slice().try_into()?;
    let public_key = recoverable.recover_verifying_key(message)?;
    let public_key: [u8; 33] = public_key.to_bytes().as_slice().try_into()?;
    let address: Address = (&public_key).try_into()?;
    Ok(address)
}

pub(crate) struct PackageHandler;
impl PackageHandler {
    /// Get a package.
    pub(crate) async fn get(
        Extension(state): Extension<ServerState>,
        Path((address, name, version)): Path<(Address, String, Version)>,
    ) -> std::result::Result<(HeaderMap, Bytes), StatusCode> {
        let reader = state.read().await;
        let _url = reader.config.ipfs.url.clone();
        let mime_type = reader.config.registry.mime.clone();
        let kind = reader.config.registry.kind;

        tracing::debug!(
            address = %address,
            name = %name,
            version = ?version);

        // Get the package meta data
        let meta = reader
            .layers
            .get_pointer(kind, &address, &name, &version)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        tracing::debug!(meta = ?meta);

        if let Some(doc) = meta {
            let body = reader
                .layers
                .get_blob(&doc.definition.archive)
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
        Extension(state): Extension<ServerState>,
        TypedHeader(mime): TypedHeader<ContentType>,
        TypedHeader(signature): TypedHeader<Signature>,
        body: Bytes,
    ) -> std::result::Result<Json<Receipt>, StatusCode> {
        let encoded_signature = base64::encode(signature.as_ref());

        // Verify the signature header against the payload bytes
        let address = verify_signature(signature.into(), &body)
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let reader = state.read().await;

        // Check if the author is denied
        if let Some(deny) = &reader.config.registry.deny {
            if deny.contains(&address) {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }

        // Check if the author is allowed
        if let Some(allow) = &reader.config.registry.allow {
            if !allow.contains(&address) {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }

        let mime_type = reader.config.registry.mime.clone();
        let kind = reader.config.registry.kind;

        tracing::debug!(mime = ?mime_type);

        // TODO: ensure approval signatures

        let gzip: mime::Mime = mime_type
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let gzip_ct = ContentType::from(gzip);

        if mime == gzip_ct {
            let (descriptor, package_meta) = PackageReader::read(kind, &body)
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            // Check the package version does not already exist
            let meta = reader
                .layers
                .get_pointer(
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

            let id = reader
                .layers
                .add_blob(body)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            tracing::debug!(id = %id, "added package");

            // Store the package meta data
            let receipt = reader
                .layers
                .add_pointer(
                    kind,
                    encoded_signature,
                    &address,
                    descriptor,
                    id,
                    package_meta,
                )
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            Ok(Json(receipt))
        } else {
            Err(StatusCode::BAD_REQUEST)
        }
    }
}
