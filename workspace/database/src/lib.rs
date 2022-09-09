mod error;
mod model;
mod value_objects;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;

pub use model::*;
pub use value_objects::*;
