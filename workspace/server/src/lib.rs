use axum_server::Handle;
use std::{net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc};
use tokio::sync::RwLock;

mod config;
mod error;
mod handlers;
mod headers;
mod layer;
mod server;

pub type Result<T> = std::result::Result<T, error::Error>;

pub use config::ServerConfig;
pub use error::Error;
pub(crate) use layer::Layers;
pub use server::{Server, ServerInfo};

fn get_layer(
    config: &config::LayerConfig,
    registry: &config::RegistryConfig,
) -> Result<Box<dyn layer::Layer + Send + Sync + 'static>> {
    match config {
        config::LayerConfig::Ipfs { url } => {
            Ok(Box::new(layer::ipfs::IpfsLayer::new(url)?))
        }
        config::LayerConfig::Aws {
            profile,
            region,
            bucket,
        } => Ok(Box::new(layer::s3::S3Layer::new(
            profile.to_string(),
            region.to_string(),
            bucket.to_string(),
            registry.mime.clone(),
        )?)),
    }
}

fn build_layers(config: &ServerConfig) -> Result<Layers> {
    let storage = get_layer(&config.storage, &config.registry)?;

    let mirror = if let Some(mirror) = &config.mirror {
        Some(get_layer(mirror, &config.registry)?)
    } else {
        None
    };

    Ok(Layers {
        storage,
        mirror,
    })
}

/// Start a server using the given bind address and configuration.
pub async fn start(bind: String, config: PathBuf) -> Result<()> {
    let name = env!("CARGO_PKG_NAME").to_string();
    let version = env!("CARGO_PKG_VERSION").to_string();
    let config = ServerConfig::load(&config)?;
    let layers = build_layers(&config)?;
    let handle = Handle::new();
    let state = Arc::new(RwLock::new(server::State {
        info: ServerInfo { name, version },
        config,
        layers,
    }));
    let addr = SocketAddr::from_str(&bind)?;
    let server: Server = Default::default();
    server.start(addr, state, handle).await?;
    Ok(())
}
