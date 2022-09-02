use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    /// Error generated by the client library.
    #[error(transparent)]
    Client(#[from] ipfs_registry_client::Error),

    /// Error generated by the server library.
    #[error(transparent)]
    Server(#[from] ipfs_registry_server::Error),

    /// Error generated by the io module.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Error generated by the JSON library.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
