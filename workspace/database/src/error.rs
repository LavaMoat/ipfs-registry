use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {

    /// Error generated converting from a slice.
    #[error(transparent)]
    TryFromSlice(#[from] std::array::TryFromSliceError),

    #[error(transparent)]
    Sql(#[from] sqlx::Error),
    //#[error(transparent)]
    //Migrate(#[from] sqlx::migrate::MigrateError),
}
