use axum::{
    body::Bytes,
    extract::{Extension, Query, TypedHeader},
    headers::ContentType,
    http::{HeaderMap, StatusCode},
    Json,
};

//use axum_macros::debug_handler;

use k256::ecdsa::recoverable;
use serde::Deserialize;
use sha3::{Digest, Sha3_256};

use web3_address::ethereum::Address;

use ipfs_registry_core::{
    Artifact, Definition, ObjectKey, PackageKey, PackageMeta, PackageReader,
    PackageSignature, Pointer, Receipt,
};

use crate::{headers::Signature, layer::Layer, server::ServerState, Result};

#[derive(Debug, Deserialize)]
pub struct PackageQuery {
    id: PackageKey,
}

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
        Query(query): Query<PackageQuery>,
    ) -> std::result::Result<(HeaderMap, Bytes), StatusCode> {
        let mime_type = state.config.registry.mime.clone();
        let kind = state.config.registry.kind;

        match query.id {
            PackageKey::Pointer(address, name, version) => {
                tracing::debug!(
                    address = %address,
                    name = %name,
                    version = ?version);

                let descriptor = Artifact {
                    kind,
                    namespace: address.to_string(),
                    package: PackageMeta { name, version },
                };

                // Get the package pointer
                let pointer = state
                    .layers
                    .get_pointer(&descriptor)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                tracing::debug!(pointer = ?pointer);

                if let Some(doc) = pointer {
                    let body = state
                        .layers
                        .get_blob(&doc.definition.object)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    // Verify the checksum
                    let checksum = Sha3_256::digest(&body);
                    if checksum.as_slice()
                        != doc.definition.checksum.as_slice()
                    {
                        return Err(StatusCode::UNPROCESSABLE_ENTITY);
                    }

                    // Verify the signature
                    let signature = doc.definition.signature;
                    let signature_bytes = base64::decode(signature.value)
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    let signature_bytes: [u8; 65] = signature_bytes
                        .as_slice()
                        .try_into()
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    verify_signature(signature_bytes, &body)
                        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

                    let mut headers = HeaderMap::new();
                    headers
                        .insert("content-type", mime_type.parse().unwrap());

                    Ok((headers, Bytes::from(body)))
                } else {
                    Err(StatusCode::NOT_FOUND)
                }
            }
            PackageKey::Cid(cid) => {
                let key = ObjectKey::Cid(cid.to_string());
                let body = state
                    .layers
                    .get_blob(&key)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                let mut headers = HeaderMap::new();
                headers.insert("content-type", mime_type.parse().unwrap());

                Ok((headers, Bytes::from(body)))
            }
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

        // Check if the author is denied
        if let Some(deny) = &state.config.registry.deny {
            if deny.contains(&address) {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }

        // Check if the author is allowed
        if let Some(allow) = &state.config.registry.allow {
            if !allow.contains(&address) {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }

        let mime_type = state.config.registry.mime.clone();
        let kind = state.config.registry.kind;

        tracing::debug!(mime = ?mime_type);

        // TODO: ensure approval signatures

        let gzip: mime::Mime = mime_type
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let gzip_ct = ContentType::from(gzip);

        if mime == gzip_ct {
            let (package, package_meta) = PackageReader::read(kind, &body)
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            let descriptor = Artifact {
                kind,
                namespace: address.to_string(),
                package,
            };

            let artifact = descriptor.clone();

            // Check the package version does not already exist
            let meta = state
                .layers
                .get_pointer(&descriptor)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            if meta.is_some() {
                return Err(StatusCode::CONFLICT);
            }

            let checksum = Sha3_256::digest(&body);

            let mut objects = state
                .layers
                .add_blob(body, &descriptor)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            tracing::debug!(id = ?objects, "added package");

            let object = objects.remove(0);

            let definition = Definition {
                artifact: descriptor,
                object,
                signature: PackageSignature {
                    signer: address,
                    value: encoded_signature,
                },
                checksum: checksum.to_vec(),
            };

            let doc = Pointer {
                definition,
                package: package_meta,
            };

            // Store the package meta data
            let pointers = state
                .layers
                .add_pointer(doc)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let receipt = Receipt { pointers, artifact };

            Ok(Json(receipt))
        } else {
            Err(StatusCode::BAD_REQUEST)
        }
    }
}
