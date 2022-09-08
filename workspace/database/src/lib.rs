mod error;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;

use sqlx::{Database, Pool, SqlitePool};
use web3_address::ethereum::Address;

#[derive(Debug)]
pub struct NamespaceRow {
    pub name: String,
    pub owner: Address,
}

pub struct Namespace<T: Database> {
    marker: std::marker::PhantomData<T>,
}

// FIXME: use Pool<T> and fix Executor bounds

impl<T: Database> Namespace<T> {
    /// Add a namespace.
    pub async fn add(pool: &SqlitePool, name: &str, owner: &Address) -> Result<i64> {
        let mut conn = pool.acquire().await?;
        let addr = owner.as_ref();

        let id = sqlx::query!(
            r#"
                INSERT INTO namespaces ( name, owner )
                VALUES ( ?1, ?2 )
            "#,
            name, addr,
        )
        .execute(&mut conn)
        .await?
        .last_insert_rowid();

        Ok(id)
    }

    /// Get a namespace by id.
    pub async fn get_by_id(pool: &SqlitePool, id: i64) -> Result<Option<NamespaceRow>> {
        let result = sqlx::query!(
            r#"
                SELECT name, owner
                FROM namespaces
                WHERE namespace_id = ?
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        if let Some(result) = result {
            let owner: [u8; 20] = result.owner.as_slice().try_into()?;
            let owner: Address = owner.into();
            Ok(Some(NamespaceRow { name: result.name, owner }))
        } else { Ok(None) }
    }
}
