mod error;
mod fetch;
mod keygen;
mod publish;

pub type Result<T> = std::result::Result<T, error::Error>;

pub use error::Error;

pub use fetch::fetch;
pub use keygen::keygen;
pub use publish::publish;
