use axum::{
    extract::{Extension, TypedHeader},
    http::StatusCode,
};

//use axum_macros::debug_handler;

use sqlx::{Database, Sqlite};

use ipfs_registry_core::WELL_KNOWN_MESSAGE;

use ipfs_registry_database::Publisher;

use crate::{
    handlers::verify_signature, headers::Signature, server::ServerState,
};

pub(crate) struct PublisherHandler<T: Database> {
    marker: std::marker::PhantomData<T>,
}

impl<T: Database> PublisherHandler<T> {
    /// Create a new publisher.
    pub(crate) async fn post(
        Extension(state): Extension<ServerState<T>>,
        TypedHeader(signature): TypedHeader<Signature>,
    ) -> std::result::Result<StatusCode, StatusCode> {
        // Verify the signature header against the well known message
        let address = verify_signature(signature.into(), WELL_KNOWN_MESSAGE)
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let record =
            Publisher::<Sqlite>::find_by_address(&state.pool, &address)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if record.is_some() {
            return Err(StatusCode::CONFLICT);
        }

        Publisher::<Sqlite>::add(&state.pool, &address)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(StatusCode::OK)
    }
}
