use anyhow::Result;
use axum_server::Handle;
use sqlx::Any;
use std::{net::SocketAddr, sync::Arc, thread};
use tokio::sync::oneshot;
use url::Url;

use k256::ecdsa::SigningKey;
use web3_address::ethereum::Address;

use ipfs_registry_server::{
    build_layers,
    config::{LayerConfig, RegistryConfig, ServerConfig, StorageConfig},
    Server, ServerInfo, State,
};

const ADDR: &str = "127.0.0.1:9009";
const SERVER: &str = "http://localhost:9009";

struct MockServer {
    handle: Handle,
}

impl MockServer {
    fn new() -> Result<Self> {
        Ok(Self {
            handle: Handle::new(),
        })
    }

    async fn start(&self, config: ServerConfig) -> Result<()> {
        let addr: SocketAddr = ADDR.parse::<SocketAddr>()?;

        tracing::info!("start mock server {:#?}", addr);

        let layers = build_layers(&config)?;

        let state = Arc::new(
            State::<Any>::new(
                config,
                ServerInfo {
                    name: String::from("integration-test"),
                    version: String::from("0.0.0"),
                },
                layers,
            )
            .await?,
        );

        let server = Server::<Any>::new();
        server.start(addr, state, self.handle.clone()).await?;
        Ok(())
    }

    /// Run the mock server in a separate thread.
    fn spawn(
        tx: oneshot::Sender<SocketAddr>,
        config: ServerConfig,
    ) -> Result<ShutdownHandle> {
        let server = MockServer::new()?;
        let listen_handle = server.handle.clone();
        let user_handle = server.handle.clone();

        thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async move {
                loop {
                    if let Some(addr) = listen_handle.listening().await {
                        tracing::info!("server has started {:#?}", addr);
                        tx.send(addr)
                            .expect("failed to send listening notification");
                        break;
                    }
                }
            });
        });

        thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                server.start(config).await.expect("failed to start server");
            });
            ()
        });

        Ok(ShutdownHandle(user_handle))
    }
}

/// Ensure the server is shutdown when the handle is dropped.
pub struct ShutdownHandle(Handle);

impl Drop for ShutdownHandle {
    fn drop(&mut self) {
        tracing::info!("shutdown mock server");
        self.0.shutdown();
    }
}

pub fn default_server_config() -> ServerConfig {
    let layer = LayerConfig::Memory { memory: true };
    let storage: StorageConfig = layer.into();
    let config = ServerConfig::new(storage);
    config
}

pub fn registry_server_config(registry: RegistryConfig) -> ServerConfig {
    let mut config = default_server_config();
    config.registry = registry;
    config
}

pub fn spawn(
    config: ServerConfig,
) -> Result<(oneshot::Receiver<SocketAddr>, ShutdownHandle)> {
    let (tx, rx) = oneshot::channel::<SocketAddr>();
    let handle = MockServer::spawn(tx, config)?;
    Ok((rx, handle))
}

pub fn server() -> Url {
    Url::parse(SERVER).expect("failed to parse server URL")
}

pub fn new_signing_key() -> (SigningKey, Address) {
    let signing_key = SigningKey::random(&mut rand::thread_rng());
    let verifying_key = signing_key.verifying_key();
    let address: Address = verifying_key.into();
    (signing_key, address)
}
