use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
    //#[error(transparent)]
    //Migrate(#[from] sqlx::migrate::MigrateError),
}
