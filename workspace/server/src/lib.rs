mod config;
mod error;
mod handlers;
mod headers;
mod server;

pub type Result<T> = std::result::Result<T, error::Error>;

pub use config::ServerConfig;
pub use error::Error;
pub use server::{Server, ServerInfo, State};
