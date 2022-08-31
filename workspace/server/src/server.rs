use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::Extension,
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use serde::Serialize;
use serde_json::json;
use tokio::sync::{RwLock, RwLockReadGuard};
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer};

use crate::{
    config::TlsConfig, handlers::PackageHandler, headers::X_SIGNATURE,
    layer::Layers, Result, ServerConfig,
};

/// Type alias for the server state.
pub(crate) type ServerState = Arc<State>;

/// Server state.
pub(crate) struct State {
    /// The server configuration.
    pub config: ServerConfig,
    /// Server information.
    pub info: ServerInfo,
    /// Storage layers.
    pub layers: Layers,
}

/// Server information.
#[derive(Serialize)]
pub struct ServerInfo {
    /// Name of the crate.
    pub name: String,
    /// Version of the crate.
    pub version: String,
}

#[derive(Default)]
pub struct Server;

impl Server {
    /// Start the server.
    pub(crate) async fn start(
        &self,
        addr: SocketAddr,
        state: ServerState,
        handle: Handle,
    ) -> Result<()> {
        let origins = Server::read_origins(&state)?;
        let limit = state.config.registry.body_limit;
        let tls = state.config.tls.as_ref().cloned();

        if let Some(tls) = tls {
            self.run_tls(addr, state, handle, origins, limit, tls).await
        } else {
            self.run(addr, state, handle, origins, limit).await
        }
    }

    /// Start the server running on HTTPS.
    async fn run_tls(
        &self,
        addr: SocketAddr,
        state: ServerState,
        handle: Handle,
        origins: Option<Vec<HeaderValue>>,
        limit: usize,
        tls: TlsConfig,
    ) -> Result<()> {
        let tls = RustlsConfig::from_pem_file(&tls.cert, &tls.key).await?;
        let app = Server::router(state, origins, limit)?;
        tracing::info!("listening on {}", addr);
        axum_server::bind_rustls(addr, tls)
            .handle(handle)
            .serve(app.into_make_service())
            .await?;
        Ok(())
    }

    /// Start the server running on HTTP.
    async fn run(
        &self,
        addr: SocketAddr,
        state: ServerState,
        handle: Handle,
        origins: Option<Vec<HeaderValue>>,
        limit: usize,
    ) -> Result<()> {
        let app = Server::router(state, origins, limit)?;
        tracing::info!("listening on {}", addr);
        axum_server::bind(addr)
            .handle(handle)
            .serve(app.into_make_service())
            .await?;
        Ok(())
    }

    fn read_origins(
        state: &State,
    ) -> Result<Option<Vec<HeaderValue>>> {
        if let Some(cors) = &state.config.cors {
            let mut origins = Vec::new();
            for url in cors.origins.iter() {
                origins.push(HeaderValue::from_str(
                    url.as_str().trim_end_matches('/'),
                )?);
            }
            Ok(Some(origins))
        } else {
            Ok(None)
        }
    }

    fn router(
        state: ServerState,
        origins: Option<Vec<HeaderValue>>,
        limit: usize,
    ) -> Result<Router> {
        let cors = if let Some(origins) = origins {
            CorsLayer::new()
                .allow_methods(vec![Method::GET, Method::POST])
                .allow_headers(vec![
                    AUTHORIZATION,
                    CONTENT_TYPE,
                    X_SIGNATURE.clone(),
                ])
                .allow_origin(origins)
        } else {
            CorsLayer::very_permissive()
        };

        let app = Router::new()
            .route("/api", get(api))
            .route(
                "/api/package/:address/:name/:version",
                get(PackageHandler::get),
            )
            .route("/api/package", put(PackageHandler::put))
            .layer(RequestBodyLimitLayer::new(limit))
            .layer(cors)
            .layer(Extension(state));

        Ok(app)
    }
}

/// Serve the API identity page.
pub(crate) async fn api(
    Extension(state): Extension<Arc<RwLock<State>>>,
) -> impl IntoResponse {
    let reader = state.read().await;
    Json(json!(&reader.info))
}
