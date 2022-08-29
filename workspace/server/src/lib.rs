use axum_server::Handle;
use std::{net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc};
use tokio::sync::RwLock;

mod config;
mod error;
mod handlers;
mod headers;
mod server;

pub type Result<T> = std::result::Result<T, error::Error>;

pub use config::ServerConfig;
pub use error::Error;
pub use server::{Server, ServerInfo, State};

/// Start a server using the given bind address and configuration.
pub async fn start(bind: String, config: PathBuf) -> Result<()> {
    let name = env!("CARGO_PKG_NAME").to_string();
    let version = env!("CARGO_PKG_VERSION").to_string();
    let config = ServerConfig::load(&config)?;
    let handle = Handle::new();
    let state = Arc::new(RwLock::new(State {
        info: ServerInfo { name, version },
        config,
    }));
    let addr = SocketAddr::from_str(&bind)?;
    let server: Server = Default::default();
    server.start(addr, state, handle).await?;
    Ok(())
}
