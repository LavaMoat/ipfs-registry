use clap::Parser;

use axum_server::Handle;
use std::{net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc};
use tokio::sync::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use ipfs_registry_server::{Server, Result, ServerConfig, State, ServerInfo};

/// Signed package registry server.
#[derive(Parser, Debug)]
#[clap(name = "ipkg-server", author, version, about, long_about = None)]
struct Cli {
    /// Bind to host:port.
    #[clap(short, long, default_value = "127.0.0.1:9060")]
    bind: String,

    /// Config file to load.
    #[clap(short, long, parse(from_os_str))]
    config: PathBuf,
}

async fn run() -> Result<()> {
    let args = Cli::parse();

    let name = env!("CARGO_PKG_NAME").to_string();
    let version = env!("CARGO_PKG_VERSION").to_string();

    let mut config = ServerConfig::load(&args.config)?;

    let handle = Handle::new();

    let state = Arc::new(RwLock::new(State {
        info: ServerInfo { name, version },
        config,
    }));

    let addr = SocketAddr::from_str(&args.bind)?;
    let server = Server::new();
    server.start(addr, state, handle).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| {
                "info".into()
            }),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    match run().await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("{}", e);
        }
    }
    Ok(())
}
