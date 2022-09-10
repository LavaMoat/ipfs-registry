use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::Extension,
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use serde::Serialize;
use serde_json::json;
use tower_http::{
    cors::CorsLayer, limit::RequestBodyLimitLayer, trace::TraceLayer,
};

use sqlx::{Any, Database, Pool, Sqlite, SqlitePool};

use crate::{
    config::ServerConfig,
    config::TlsConfig,
    handlers::{NamespaceHandler, PackageHandler, PublisherHandler},
    headers::X_SIGNATURE,
    layer::Layers,
    Result,
};

/// Type alias for the server state.
pub(crate) type ServerState<T> = Arc<State<T>>;

/// Server state.
pub struct State<T: Database> {
    /// The server configuration.
    pub(crate) config: ServerConfig,
    /// Server information.
    pub(crate) info: ServerInfo,
    /// Storage layers.
    pub(crate) layers: Layers,
    /// Connection pool.
    pub(crate) pool: Pool<Sqlite>,

    // Keeping the generics so that later we
    // can use the Any support in sqlx
    marker: std::marker::PhantomData<T>,
}

impl State<Any> {
    /// Create a new state.
    pub async fn new(
        config: ServerConfig,
        info: ServerInfo,
        layers: Layers,
    ) -> Result<State<Sqlite>> {
        let url = std::env::var("DATABASE_URL")
            .ok()
            .unwrap_or_else(|| config.database.url.clone());

        tracing::info!(db = %url);

        //let pool = AnyPool::connect(&url).await?;
        let pool = SqlitePool::connect(&url).await?;

        if &config.database.url == "sqlite::memory:" {
            sqlx::migrate!("../../migrations").run(&pool).await?;
        }

        Ok(State::<Sqlite> {
            config,
            info,
            layers,
            pool,
            marker: std::marker::PhantomData,
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

#[derive(Default)]
pub struct Server<T: Database> {
    marker: std::marker::PhantomData<T>,
}

impl Server<Any> {
    pub fn new() -> Server<Sqlite> {
        Server::<Sqlite> {
            marker: std::marker::PhantomData,
        }
    }
}

impl<T: Database + Send + Sync> Server<T> {
    /// Start the server.
    pub async fn start(
        &self,
        addr: SocketAddr,
        state: ServerState<T>,
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
        state: ServerState<T>,
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
        state: ServerState<T>,
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
        state: &State<T>,
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
        state: ServerState<T>,
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
            .route("/api", get(ApiHandler::<Sqlite>::get))
            .route("/api/publisher", post(PublisherHandler::<Sqlite>::post))
            .route(
                "/api/namespace/:namespace",
                post(NamespaceHandler::<Sqlite>::post),
            )
            .route("/api/package", get(PackageHandler::<Sqlite>::get))
            .route(
                "/api/package/:namespace",
                put(PackageHandler::<Sqlite>::put),
            )
            //.route("/api/package", put(PackageHandler::put))
            .layer(RequestBodyLimitLayer::new(limit))
            .layer(cors)
            .layer(TraceLayer::new_for_http())
            .layer(Extension(state));

        Ok(app)
    }
}

pub(crate) struct ApiHandler<T: Database> {
    marker: std::marker::PhantomData<T>,
}

impl<T: Database + Send + Sync> ApiHandler<T> {
    /// Serve the API identity page.
    pub(crate) async fn get(
        Extension(state): Extension<ServerState<T>>,
    ) -> impl IntoResponse {
        Json(json!(&state.info))
    }
}
