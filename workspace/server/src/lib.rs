use axum_server::Handle;
use sqlx::Any;
use std::{net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc};

pub mod config;
mod error;
mod handlers;
mod headers;
mod layer;
mod server;

pub type Result<T> = std::result::Result<T, error::Error>;

pub use error::Error;
pub use layer::build as build_layers;
pub use server::{Server, ServerInfo, State};

/// Start a server using the given bind address and configuration.
pub async fn start(bind: String, config: PathBuf) -> Result<()> {
    let name = env!("CARGO_PKG_NAME").to_string();
    let version = env!("CARGO_PKG_VERSION").to_string();
    let config = config::ServerConfig::load(&config)?;
    let layers = layer::build(&config)?;
    let handle = Handle::new();
    let state = Arc::new(
        server::State::<Any>::new(
            config,
            ServerInfo { name, version },
            layers,
        )
        .await?,
    );
    let addr = SocketAddr::from_str(&bind)?;
    let server = Server::<Any>::new();
    server.start(addr, state, handle).await?;
    Ok(())
}
