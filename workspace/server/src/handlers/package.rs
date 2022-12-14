use axum::{
    body::Bytes,
    extract::{Extension, Path, Query, TypedHeader},
    headers::ContentType,
    http::{HeaderMap, StatusCode},
    Json,
};

//use axum_macros::debug_handler;

use semver::VersionReq;
use serde::Deserialize;
use sha3::{Digest, Sha3_256};

use ipfs_registry_core::{
    Artifact, Definition, Namespace, ObjectKey, PackageKey, PackageName,
    PackageReader, PackageSignature, Pointer, Receipt,
};

use ipfs_registry_database::{
    default_limit, Error as DatabaseError, NamespaceModel, PackageModel,
    PackageRecord, Pager, ResultSet, SortOrder, VersionIncludes,
    VersionRecord,
};

use crate::{
    handlers::{
        verify_signature,
        webhooks::{
            execute_webhooks, WebHookBody, WebHookEvent, WebHookPacket,
        },
    },
    headers::Signature,
    server::ServerState,
};

#[derive(Debug, Deserialize)]
pub struct PackageQuery {
    id: PackageKey,
}

#[derive(Default, Debug, Deserialize)]
#[serde(default)]
pub struct ListPackagesQuery {
    include: VersionIncludes,
    // NOTE: cannot use #[serde(flatten)]
    // SEE: https://github.com/tokio-rs/axum/issues/1366
    offset: i64,
    #[serde(default = "default_limit")]
    limit: i64,
    sort: SortOrder,
}

impl ListPackagesQuery {
    fn into_pager(&self) -> Pager {
        Pager {
            offset: self.offset,
            limit: self.limit,
            sort: self.sort,
        }
    }
}

#[derive(Default, Debug, Deserialize)]
#[serde(default)]
pub struct ListVersionsQuery {
    range: Option<VersionReq>,
    offset: i64,
    #[serde(default = "default_limit")]
    limit: i64,
    sort: SortOrder,
}

impl ListVersionsQuery {
    fn into_pager(&self) -> Pager {
        Pager {
            offset: self.offset,
            limit: self.limit,
            sort: self.sort,
        }
    }
}

#[derive(Default, Debug, Deserialize)]
#[serde(default)]
pub struct LatestQuery {
    prerelease: bool,
}

pub(crate) struct PackageHandler;

impl PackageHandler {
    /// Get a package record.
    pub(crate) async fn get_package(
        Extension(state): Extension<ServerState>,
        Path((namespace, package)): Path<(Namespace, PackageName)>,
    ) -> std::result::Result<Json<PackageRecord>, StatusCode> {
        let namespace_record =
            NamespaceModel::find_by_name(&state.pool, &namespace)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .ok_or(StatusCode::NOT_FOUND)?;

        let package_record = PackageModel::find_by_name(
            &state.pool,
            namespace_record.namespace_id,
            &package,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

        Ok(Json(package_record))
    }

    /// List packages for a namespace.
    pub(crate) async fn list_packages(
        Extension(state): Extension<ServerState>,
        Path(namespace): Path<Namespace>,
        Query(query): Query<ListPackagesQuery>,
    ) -> std::result::Result<Json<ResultSet<PackageRecord>>, StatusCode> {
        let pager = query.into_pager();

        match PackageModel::list_packages(
            &state.pool,
            &namespace,
            &pager,
            query.include,
        )
        .await
        {
            Ok(records) => Ok(Json(records)),
            Err(e) => Err(match e {
                DatabaseError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }),
        }
    }

    /// List versions for a namespace and package.
    pub(crate) async fn list_versions(
        Extension(state): Extension<ServerState>,
        Path((namespace, package)): Path<(Namespace, PackageName)>,
        Query(query): Query<ListVersionsQuery>,
    ) -> std::result::Result<Json<ResultSet<VersionRecord>>, StatusCode> {
        let pager = query.into_pager();

        if let Some(range) = query.range {
            match PackageModel::find_versions(
                &state.pool,
                &namespace,
                &package,
                &range,
                &pager,
            )
            .await
            {
                Ok(records) => Ok(Json(records)),
                Err(e) => Err(match e {
                    DatabaseError::NotFound(_) => StatusCode::NOT_FOUND,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                }),
            }
        } else {
            match PackageModel::list_versions(
                &state.pool,
                &namespace,
                &package,
                &pager,
            )
            .await
            {
                Ok(records) => Ok(Json(records)),
                Err(e) => Err(match e {
                    DatabaseError::NotFound(_) => StatusCode::NOT_FOUND,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                }),
            }
        }
    }

    /// Get the latest version of a package.
    pub(crate) async fn latest_version(
        Extension(state): Extension<ServerState>,
        Path((namespace, package)): Path<(Namespace, PackageName)>,
        Query(latest): Query<LatestQuery>,
    ) -> std::result::Result<Json<VersionRecord>, StatusCode> {
        match PackageModel::find_latest_by_name(
            &state.pool,
            &namespace,
            &package,
            latest.prerelease,
        )
        .await
        {
            Ok(record) => {
                let record = record.ok_or_else(|| StatusCode::NOT_FOUND)?;
                Ok(Json(record))
            }
            Err(e) => Err(match e {
                DatabaseError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }),
        }
    }

    /// Get the exact version of a package.
    pub(crate) async fn exact_version(
        Extension(state): Extension<ServerState>,
        Query(query): Query<PackageQuery>,
    ) -> std::result::Result<Json<VersionRecord>, StatusCode> {
        match PackageModel::find_by_key(&state.pool, &query.id).await {
            Ok((_, _, record)) => {
                let record = record.ok_or_else(|| StatusCode::NOT_FOUND)?;
                Ok(Json(record))
            }
            Err(e) => Err(match e {
                DatabaseError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }),
        }
    }

    /// Deprecate a package.
    pub(crate) async fn deprecate(
        Extension(state): Extension<ServerState>,
        TypedHeader(signature): TypedHeader<Signature>,
        Path((namespace, package)): Path<(Namespace, PackageName)>,
        body: Bytes,
    ) -> std::result::Result<StatusCode, StatusCode> {
        let address = verify_signature(signature.into(), &body)
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let message = std::str::from_utf8(&body)
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        match PackageModel::deprecate(
            &state.pool,
            &address,
            &namespace,
            &package,
            &message,
        )
        .await
        {
            Ok(_) => Ok(StatusCode::OK),
            Err(e) => Err(match e {
                DatabaseError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
                DatabaseError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }),
        }
    }

    /// Yank a version of a package.
    pub(crate) async fn yank(
        Extension(state): Extension<ServerState>,
        TypedHeader(signature): TypedHeader<Signature>,
        Query(query): Query<PackageQuery>,
        body: Bytes,
    ) -> std::result::Result<StatusCode, StatusCode> {
        let address = verify_signature(signature.into(), &body)
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let message = std::str::from_utf8(&body)
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        match PackageModel::yank(&state.pool, &address, &query.id, &message)
            .await
        {
            Ok(_) => Ok(StatusCode::OK),
            Err(e) => Err(match e {
                DatabaseError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
                DatabaseError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }),
        }
    }

    /// Download a package.
    pub(crate) async fn fetch(
        Extension(state): Extension<ServerState>,
        Query(query): Query<PackageQuery>,
    ) -> std::result::Result<(HeaderMap, Bytes), StatusCode> {
        let mime_type = state.config.registry.mime.clone();
        let _kind = state.config.registry.kind;

        match PackageModel::find_by_key(&state.pool, &query.id).await {
            Ok((_, _, record)) => {
                let record = record.ok_or(StatusCode::NOT_FOUND)?;

                let body = state
                    .layers
                    .fetch(&record.pointer_id, record.content_id.as_ref())
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                // Verify the checksum
                let checksum = Sha3_256::digest(&body);
                if checksum.as_slice() != record.checksum.as_slice() {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }

                verify_signature(record.signature, &body)
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

                let mut headers = HeaderMap::new();
                headers.insert("content-type", mime_type.parse().unwrap());

                if let Some(hooks) = state.config.webhooks.clone() {
                    let body = WebHookBody { inner: record };
                    let packet = WebHookPacket {
                        event: WebHookEvent::Fetch,
                        body,
                    };
                    tokio::spawn(execute_webhooks(hooks, packet));
                }

                Ok((headers, Bytes::from(body)))
            }
            Err(e) => Err(match e {
                DatabaseError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }),
        }
    }

    /// Publish a new package.
    pub(crate) async fn publish(
        Extension(state): Extension<ServerState>,
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
        match NamespaceModel::can_access_namespace(
            &state.pool,
            &address,
            &namespace,
        )
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
                match PackageModel::can_publish_package(
                    &state.pool,
                    &address,
                    &namespace_record,
                    &package.name,
                    Some(&package.version),
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

                        let objects = state
                            .layers
                            .publish(body, &descriptor)
                            .await
                            .map_err(|e| {
                                tracing::error!("{}", e);
                                StatusCode::INTERNAL_SERVER_ERROR
                            })?;

                        tracing::debug!(id = ?objects, "added package");

                        // Direct key for the publish receipt
                        let key = objects.iter().find_map(|o| {
                            if let ObjectKey::Cid(value) = o {
                                Some(PackageKey::Cid(*value))
                            } else {
                                None
                            }
                        });

                        let checksum: [u8; 32] = checksum
                            .as_slice()
                            .try_into()
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                        let doc = Pointer {
                            definition: Definition {
                                artifact: descriptor,
                                objects,
                                signature: PackageSignature {
                                    signer: address,
                                    value: signature.into(),
                                },
                                checksum,
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

                        let receipt = Receipt {
                            id,
                            artifact,
                            key,
                            checksum,
                        };

                        if let Some(hooks) = state.config.webhooks.clone() {
                            let body = WebHookBody { inner: doc };
                            let packet = WebHookPacket {
                                event: WebHookEvent::Publish,
                                body,
                            };
                            tokio::spawn(execute_webhooks(hooks, packet));
                        }

                        Ok(Json(receipt))
                    }
                    Err(e) => Err(match e {
                        DatabaseError::PackageExists(_, _, _)
                        | DatabaseError::VersionNotAhead(_, _) => {
                            StatusCode::CONFLICT
                        }
                        _ => StatusCode::INTERNAL_SERVER_ERROR,
                    }),
                }
            }
            Err(e) => Err(match e {
                DatabaseError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
                DatabaseError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }),
        }
    }
}
