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
use tower_http::{cors::{CorsLayer}, limit::RequestBodyLimitLayer};

use crate::{
    config::TlsConfig, handlers::PackageHandler, Result, ServerConfig,
};

/// Server state.
pub struct State {
    /// The server configuration.
    pub config: ServerConfig,
    /// Server information.
    pub info: ServerInfo,
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
    pub async fn start(
        &self,
        addr: SocketAddr,
        state: Arc<RwLock<State>>,
        handle: Handle,
    ) -> Result<()> {
        let reader = state.read().await;
        let origins = Server::read_origins(&reader)?;
        let limit = reader.config.registry.body_limit;
        let tls = reader.config.tls.as_ref().cloned();
        drop(reader);

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
        state: Arc<RwLock<State>>,
        handle: Handle,
        origins: Vec<HeaderValue>,
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
        state: Arc<RwLock<State>>,
        handle: Handle,
        origins: Vec<HeaderValue>,
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
        reader: &RwLockReadGuard<'_, State>,
    ) -> Result<Vec<HeaderValue>> {
        let mut origins = Vec::new();
        for url in reader.config.api.origins.iter() {
            origins.push(HeaderValue::from_str(
                url.as_str().trim_end_matches('/'),
            )?);
        }
        Ok(origins)
    }

    fn router(
        state: Arc<RwLock<State>>,
        origins: Vec<HeaderValue>,
        limit: usize,
    ) -> Result<Router> {
        let cors = CorsLayer::new()
            .allow_methods(vec![Method::GET, Method::POST])
            .allow_credentials(true)
            .allow_headers(vec![AUTHORIZATION, CONTENT_TYPE])
            .expose_headers(vec![])
            .allow_origin(origins);

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
