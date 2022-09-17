use axum::{
    extract::{Extension, Path, Query, TypedHeader},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use web3_address::ethereum::Address;

use ipfs_registry_core::{Namespace, PackageName};
use ipfs_registry_database::{
    Error as DatabaseError, NamespaceModel, NamespaceRecord, PublisherModel,
};

use crate::{
    handlers::verify_signature, headers::Signature, server::ServerState,
};

#[derive(Default, Debug, Deserialize)]
#[serde(default)]
pub struct AddUserQuery {
    admin: Option<bool>,
    package: Option<PackageName>,
}

pub(crate) struct NamespaceHandler;

impl NamespaceHandler {
    /// Create a new namespace.
    pub(crate) async fn register(
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

    /// Get a namespace record.
    pub(crate) async fn get_namespace(
        Extension(state): Extension<ServerState>,
        Path(namespace): Path<Namespace>,
    ) -> std::result::Result<Json<NamespaceRecord>, StatusCode> {
        let namespace_record =
            NamespaceModel::find_by_name(&state.pool, &namespace)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .ok_or(StatusCode::NOT_FOUND)?;
        Ok(Json(namespace_record))
    }

    /// Add a user to a namespace.
    pub(crate) async fn add_user(
        Extension(state): Extension<ServerState>,
        TypedHeader(signature): TypedHeader<Signature>,
        Path((namespace, user)): Path<(Namespace, Address)>,
        Query(query): Query<AddUserQuery>,
    ) -> std::result::Result<StatusCode, StatusCode> {
        let caller = verify_signature(signature.into(), user.as_ref())
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let admin = query.admin.is_some() && query.admin.unwrap();
        let restrictions = if let Some(package) = &query.package {
            vec![package]
        } else {
            vec![]
        };

        match NamespaceModel::add_user(
            &state.pool,
            &namespace,
            &caller,
            &user,
            admin,
            restrictions,
        )
        .await
        {
            Ok(_) => Ok(StatusCode::OK),
            Err(e) => Err(match e {
                DatabaseError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
                DatabaseError::NotFound(_) => StatusCode::NOT_FOUND,
                DatabaseError::UserExists(_, _) => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }),
        }
    }

    /// Remove a user from a namespace.
    pub(crate) async fn remove_user(
        Extension(state): Extension<ServerState>,
        TypedHeader(signature): TypedHeader<Signature>,
        Path((namespace, user)): Path<(Namespace, Address)>,
    ) -> std::result::Result<StatusCode, StatusCode> {
        let caller = verify_signature(signature.into(), user.as_ref())
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        match NamespaceModel::remove_user(
            &state.pool,
            &namespace,
            &caller,
            &user,
        )
        .await
        {
            Ok(_) => Ok(StatusCode::OK),
            Err(e) => Err(match e {
                DatabaseError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
                DatabaseError::NotFound(_) => StatusCode::NOT_FOUND,
                DatabaseError::UserExists(_, _) => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }),
        }
    }

    /// Grant a user access to a package.
    pub(crate) async fn grant_access(
        Extension(state): Extension<ServerState>,
        TypedHeader(signature): TypedHeader<Signature>,
        Path((namespace, user, package)): Path<(
            Namespace,
            Address,
            PackageName,
        )>,
    ) -> std::result::Result<StatusCode, StatusCode> {
        let caller = verify_signature(signature.into(), user.as_ref())
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        match NamespaceModel::grant_access(
            &state.pool,
            &namespace,
            &package,
            &caller,
            &user,
        )
        .await
        {
            Ok(_) => Ok(StatusCode::OK),
            Err(e) => Err(match e {
                DatabaseError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
                DatabaseError::NotFound(_) => StatusCode::NOT_FOUND,
                DatabaseError::AccessRestrictionExists(_, _) => {
                    StatusCode::CONFLICT
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }),
        }
    }

    /// Revoke user access to a package.
    pub(crate) async fn revoke_access(
        Extension(state): Extension<ServerState>,
        TypedHeader(signature): TypedHeader<Signature>,
        Path((namespace, user, package)): Path<(
            Namespace,
            Address,
            PackageName,
        )>,
    ) -> std::result::Result<StatusCode, StatusCode> {
        let caller = verify_signature(signature.into(), user.as_ref())
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        match NamespaceModel::revoke_access(
            &state.pool,
            &namespace,
            &package,
            &caller,
            &user,
        )
        .await
        {
            Ok(_) => Ok(StatusCode::OK),
            Err(e) => Err(match e {
                DatabaseError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
                DatabaseError::NotFound(_) => StatusCode::NOT_FOUND,
                DatabaseError::AccessRestrictionMissing(_, _) => {
                    StatusCode::CONFLICT
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }),
        }
    }
}
