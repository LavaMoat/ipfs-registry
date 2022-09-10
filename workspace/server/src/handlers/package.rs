use axum::{
    body::Bytes,
    extract::{Extension, Query, TypedHeader},
    headers::ContentType,
    http::{HeaderMap, StatusCode},
    Json,
};

//use axum_macros::debug_handler;

use serde::Deserialize;
use sha3::{Digest, Sha3_256};

use sqlx::Database;

use ipfs_registry_core::{
    Artifact, Definition, Namespace, ObjectKey, PackageKey, PackageMeta,
    PackageReader, PackageSignature, Pointer, Receipt,
};

use crate::{
    handlers::verify_signature, headers::Signature, layer::Layer,
    server::ServerState,
};

#[derive(Debug, Deserialize)]
pub struct PackageQuery {
    id: PackageKey,
}

pub(crate) struct PackageHandler<T: Database> {
    marker: std::marker::PhantomData<T>,
}

impl<T: Database> PackageHandler<T> {
    /// Get a package.
    pub(crate) async fn get(
        Extension(state): Extension<ServerState<T>>,
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

                let namespace: Namespace = address
                    .to_string()
                    .parse()
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                let descriptor = Artifact {
                    kind,
                    namespace,
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
                let key = ObjectKey::Cid(cid);
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
        Extension(state): Extension<ServerState<T>>,
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

        // Check MIME type is correct
        let gzip: mime::Mime = mime_type
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let gzip_ct = ContentType::from(gzip);
        if mime != gzip_ct {
            return Err(StatusCode::BAD_REQUEST);
        }

        let (package, package_meta) = PackageReader::read(kind, &body)
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let namespace: Namespace = address
            .to_string()
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let descriptor = Artifact {
            kind,
            namespace,
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

        // Direct key for the publish receipt
        let key = objects.iter().find_map(|o| {
            if let ObjectKey::Cid(value) = o {
                Some(PackageKey::Cid(value.clone()))
            } else {
                None
            }
        });

        let object = objects.remove(0);

        let doc = Pointer {
            definition: Definition {
                artifact: descriptor,
                object,
                signature: PackageSignature {
                    signer: address,
                    value: encoded_signature,
                },
                checksum: checksum.to_vec(),
            },
            package: package_meta,
        };

        // Store the package pointer document
        state
            .layers
            .add_pointer(doc)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let id = PackageKey::Pointer(
            artifact.namespace.clone(),
            artifact.package.name.clone(),
            artifact.package.version.clone(),
        );

        let receipt = Receipt { id, artifact, key };

        Ok(Json(receipt))
    }
}
