mod error;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;

use sqlx::{Database, Pool};

pub struct Namespace<T: Database> {
    marker: std::marker::PhantomData<T>,
}

impl<T: Database> Namespace<T> {
    /// Add a namespace.
    pub fn add(pool: &Pool<T>, name: String) -> Result<()> {
        todo!()
    }
}
