mod error;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;

use sqlx::{Database, Sqlite, SqlitePool};
use web3_address::ethereum::Address;

#[derive(Debug)]
pub struct NamespaceRecord {
    /// Namespace primary key.
    pub namespace_id: i64,
    /// Name for the namespace.
    pub name: String,
    /// Owner of the namespace.
    pub owner: Address,
    /// Additional publishers.
    pub publishers: Vec<Address>,
}

impl NamespaceRecord {
    /// Determine if an address is allowed to publish to
    /// this namespace.
    pub fn can_publish(&self, address: &Address) -> bool {
        if &self.owner == address {
            true
        } else {
            self.publishers.iter().find(|a| a == &address).is_some()
        }
    }
}

pub struct Publisher<T: Database> {
    marker: std::marker::PhantomData<T>,
}

impl Publisher<Sqlite> {
    /// Add a publisher.
    pub async fn add(pool: &SqlitePool, owner: &Address) -> Result<i64> {
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

    /// Add a publisher to a namespace.
    pub async fn add_publisher(
        pool: &SqlitePool,
        namespace_id: i64,
        publisher_id: i64,
    ) -> Result<i64> {
        let mut conn = pool.acquire().await?;

        let id = sqlx::query!(
            r#"
                INSERT INTO namespace_publishers ( namespace_id, publisher_id )
                VALUES ( ?1, ?2 )
            "#,
            namespace_id,
            publisher_id,
        )
        .execute(&mut conn)
        .await?
        .last_insert_rowid();

        Ok(id)
    }

    /// Get a namespace by name.
    pub async fn get_by_name(
        pool: &SqlitePool,
        name: &str,
    ) -> Result<Option<NamespaceRecord>> {
        let result = sqlx::query!(
            r#"
                SELECT
                    namespaces.namespace_id,
                    namespaces.name,
                    namespaces.publisher_id,
                    publishers.address
                FROM namespaces
                INNER JOIN publishers
                ON (namespaces.publisher_id = publishers.publisher_id)
                WHERE name = ?
            "#,
            name
        )
        .fetch_optional(pool)
        .await?;

        if let Some(result) = result {
            let owner: [u8; 20] = result.address.as_slice().try_into()?;
            let owner: Address = owner.into();

            let records = sqlx::query!(
                r#"
                    SELECT
                        namespace_publishers.namespace_id,
                        namespace_publishers.publisher_id,
                        publishers.address
                    FROM namespace_publishers
                    INNER JOIN publishers
                    ON (namespace_publishers.publisher_id = publishers.publisher_id)
                    WHERE namespace_id = ?
                "#,
                result.namespace_id,
            )
            .fetch_all(pool)
            .await?;

            let mut publishers = Vec::new();
            for result in records {
                let owner: [u8; 20] = result.address.as_slice().try_into()?;
                let owner: Address = owner.into();
                publishers.push(owner);
            }

            Ok(Some(NamespaceRecord {
                namespace_id: result.namespace_id,
                name: result.name,
                owner,
                publishers,
            }))
        } else {
            Ok(None)
        }
    }
}
