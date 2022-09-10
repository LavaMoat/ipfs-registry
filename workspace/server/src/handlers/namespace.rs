use axum::{
    extract::{Extension, Path, TypedHeader},
    http::StatusCode,
    Json,
};

use ipfs_registry_core::Namespace;
use ipfs_registry_database::{
    NamespaceModel, NamespaceRecord, PublisherModel,
};

use crate::{
    handlers::verify_signature, headers::Signature, server::ServerState,
};

pub(crate) struct NamespaceHandler;

impl NamespaceHandler {
    /// Create a new namespace.
    pub(crate) async fn post(
        Extension(state): Extension<ServerState>,
        TypedHeader(signature): TypedHeader<Signature>,
        Path(namespace): Path<Namespace>,
    ) -> std::result::Result<Json<NamespaceRecord>, StatusCode> {
        // FIXME: verify namespace is sane - no slashes!

        // Verify the signature header against supplied namespace
        let address =
            verify_signature(signature.into(), namespace.as_bytes())
                .map_err(|_| StatusCode::BAD_REQUEST)?;

        let publisher =
            PublisherModel::find_by_address(&state.pool, &address)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(publisher) = publisher {
            let record =
                NamespaceModel::find_by_name(&state.pool, &namespace)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            if record.is_some() {
                return Err(StatusCode::CONFLICT);
            }

            let record = NamespaceModel::insert_fetch(
                &state.pool,
                &namespace,
                publisher.publisher_id,
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            Ok(Json(record))
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
