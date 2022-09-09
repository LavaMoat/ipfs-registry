mod error;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;

use sqlx::{Database, Sqlite, SqlitePool};
use web3_address::ethereum::Address;

#[derive(Debug)]
pub struct NamespaceRecord {
    pub name: String,
    pub owner: Address,
}

pub struct Publisher<T: Database> {
    marker: std::marker::PhantomData<T>,
}

impl Publisher<Sqlite> {
    /// Add a publisher.
    pub async fn add(
        pool: &SqlitePool,
        owner: &Address,
    ) -> Result<i64> {
        let mut conn = pool.acquire().await?;

        let addr = owner.as_ref();

        let id = sqlx::query!(
            r#"
                INSERT INTO publishers ( address )
                VALUES ( ?1 )
            "#,
            addr,
        )
        .execute(&mut conn)
        .await?
        .last_insert_rowid();

        Ok(id)
    }
}

pub struct Namespace<T: Database> {
    marker: std::marker::PhantomData<T>,
}

impl Namespace<Sqlite> {
    /// Add a namespace.
    pub async fn add(
        pool: &SqlitePool,
        name: &str,
        publisher_id: i64,
    ) -> Result<i64> {
        let mut conn = pool.acquire().await?;

        let id = sqlx::query!(
            r#"
                INSERT INTO namespaces ( name, publisher_id )
                VALUES ( ?1, ?2 )
            "#,
            name,
            publisher_id,
        )
        .execute(&mut conn)
        .await?
        .last_insert_rowid();

        Ok(id)
    }

    /// Get a namespace by id.
    pub async fn get_by_id(
        pool: &SqlitePool,
        id: i64,
    ) -> Result<Option<NamespaceRecord>> {
        let result = sqlx::query!(
            r#"
                SELECT
                    namespaces.name,
                    namespaces.publisher_id,
                    publishers.address
                FROM namespaces
                INNER JOIN publishers
                ON (namespaces.publisher_id = publishers.publisher_id)
                WHERE namespace_id = ?
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        if let Some(result) = result {
            let owner: [u8; 20] = result.address.as_slice().try_into()?;
            let owner: Address = owner.into();
            Ok(Some(NamespaceRecord {
                name: result.name,
                owner,
            }))
        } else {
            Ok(None)
        }
    }
}
