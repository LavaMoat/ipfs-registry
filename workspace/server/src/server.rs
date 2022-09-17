use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::Extension,
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use serde::Serialize;
use serde_json::json;
use tower_http::{
    cors::CorsLayer, limit::RequestBodyLimitLayer, trace::TraceLayer,
};

use sqlx::SqlitePool;

use crate::{
    config::ServerConfig,
    config::TlsConfig,
    handlers::{NamespaceHandler, PackageHandler, PublisherHandler},
    headers::X_SIGNATURE,
    layer::Layers,
    Result,
};

/// Type alias for the server state.
pub(crate) type ServerState = Arc<State>;

/// Server state.
pub struct State {
    /// The server configuration.
    pub(crate) config: ServerConfig,
    /// Server information.
    pub(crate) info: ServerInfo,
    /// Storage layers.
    pub(crate) layers: Layers,
    /// Connection pool.
    pub(crate) pool: SqlitePool,
}

impl State {
    /// Create a new state.
    pub async fn new(
        config: ServerConfig,
        info: ServerInfo,
        layers: Layers,
    ) -> Result<State> {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .unwrap_or_else(|| config.database.url.clone());

        tracing::info!(db = %url);

        let pool = SqlitePool::connect(&url).await?;

        if &config.database.url == "sqlite::memory:" {
            sqlx::migrate!("../../migrations").run(&pool).await?;
        }

        Ok(State {
            config,
            info,
            layers,
            pool,
        })
    }
}

/// Server information.
#[derive(Serialize)]
pub struct ServerInfo {
    /// Name of the crate.
    pub name: String,
    /// Version of the crate.
    pub version: String,
}

/// Server implementation.
#[derive(Default)]
pub struct Server;

impl Server {
    /// Start the server.
    pub async fn start(
        &self,
        addr: SocketAddr,
        state: ServerState,
        handle: Handle,
    ) -> Result<()> {
        let origins = self.read_origins(&state)?;
        let limit = state.config.registry.body_limit;
        let tls = state.config.tls.as_ref().cloned();

        //sqlx::migrate!("../../migrations").run(&pool).await?;

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
        &self,
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
                .allow_methods(vec![Method::GET, Method::POST, Method::DELETE])
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
            .route("/api", get(ApiHandler::get))
            .route("/api/signup", post(PublisherHandler::signup))
            .route(
                "/api/register/:namespace",
                post(NamespaceHandler::register),
            )
            .route(
                "/api/namespace/:namespace/user/:address",
                post(NamespaceHandler::add_user)
                    .delete(NamespaceHandler::remove_user),
            )
            .route(
                "/api/namespace/:namespace/user/:address/access/:package",
                post(NamespaceHandler::grant_access)
                    .delete(NamespaceHandler::revoke_access),
            )
            .route("/api/package", get(PackageHandler::fetch))
            .route(
                "/api/package/:namespace",
                post(PackageHandler::publish)
                    .get(NamespaceHandler::get_namespace),
            )
            .route(
                "/api/package/:namespace/packages",
                get(PackageHandler::list_packages),
            )
            .route(
                "/api/package/:namespace/:package",
                get(PackageHandler::get_package),
            )
            .route(
                "/api/package/:namespace/:package/versions",
                get(PackageHandler::list_versions),
            )
            .route(
                "/api/package/:namespace/:package/latest",
                get(PackageHandler::latest_version),
            )
            .route(
                "/api/package/:namespace/:package/deprecate",
                post(PackageHandler::deprecate),
            )
            .route("/api/package/version", get(PackageHandler::exact_version))
            .route("/api/package/yank", post(PackageHandler::yank))
            .layer(RequestBodyLimitLayer::new(limit))
            .layer(cors)
            .layer(TraceLayer::new_for_http())
            .layer(Extension(state));

        Ok(app)
    }
}

pub(crate) struct ApiHandler;

impl ApiHandler {
    /// Serve the API identity page.
    pub(crate) async fn get(
        Extension(state): Extension<ServerState>,
    ) -> impl IntoResponse {
        Json(json!(&state.info))
    }
}
