use axum::{
    body::Bytes,
    extract::{Extension, Path, Query, TypedHeader},
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

use ipfs_registry_database::{Error as DatabaseError, PackageModel};

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

        match PackageModel::find_by_key(&state.pool, &query.id).await {
            Ok(version) => {
                let version_record = version.ok_or(StatusCode::NOT_FOUND)?;

                let body = state
                    .layers
                    .get_blob(&version_record.content_id)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                // Verify the checksum
                let checksum = Sha3_256::digest(&body);
                if checksum.as_slice() != version_record.checksum.as_slice() {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }

                verify_signature(version_record.signature, &body)
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

                let mut headers = HeaderMap::new();
                headers.insert("content-type", mime_type.parse().unwrap());
                Ok((headers, Bytes::from(body)))
            }
            Err(e) => Err(match e {
                DatabaseError::UnknownNamespace(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }),
        }
    }

    /// Create a new package.
    pub(crate) async fn put(
        Extension(state): Extension<ServerState<T>>,
        TypedHeader(mime): TypedHeader<ContentType>,
        TypedHeader(signature): TypedHeader<Signature>,
        Path(namespace): Path<Namespace>,
        body: Bytes,
    ) -> std::result::Result<Json<Receipt>, StatusCode> {
        //let encoded_signature = base64::encode(signature.as_ref());

        // Verify the signature header against the payload bytes
        let address = verify_signature(signature.clone().into(), &body)
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

        // Check the publisher and namespace exist and this address
        // is allowed to publish to the target namespace
        match PackageModel::verify_publish(&state.pool, &address, &namespace)
            .await
        {
            Ok((publisher_record, namespace_record)) => {
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

                let (package, package_meta) =
                    PackageReader::read(kind, &body)
                        .map_err(|_| StatusCode::BAD_REQUEST)?;

                // Check the package does not already exist
                match PackageModel::assert_publish_safe(
                    &state.pool,
                    &namespace_record,
                    &package.name,
                    &package.version,
                )
                .await
                {
                    Ok(_) => {
                        let descriptor = Artifact {
                            kind,
                            namespace,
                            package,
                        };

                        let artifact = descriptor.clone();

                        let checksum = Sha3_256::digest(&body);

                        let mut objects = state
                            .layers
                            .add_blob(body, &descriptor)
                            .await
                            .map_err(|e| {
                                tracing::error!("{}", e);
                                StatusCode::INTERNAL_SERVER_ERROR
                            })?;

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
                                    value: signature.into(),
                                },
                                checksum: checksum.to_vec(),
                            },
                            package: package_meta,
                        };

                        PackageModel::insert(
                            &state.pool,
                            &publisher_record,
                            &namespace_record,
                            &address,
                            &doc,
                        )
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
                    Err(e) => Err(match e {
                        DatabaseError::PackageExists(_, _, _) => {
                            StatusCode::CONFLICT
                        }
                        _ => StatusCode::INTERNAL_SERVER_ERROR,
                    }),
                }
            }
            Err(e) => Err(match e {
                DatabaseError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
                DatabaseError::UnknownPublisher(_)
                | DatabaseError::UnknownNamespace(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }),
        }
    }
}
