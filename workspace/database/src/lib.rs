mod error;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;

use cid::Cid;
use semver::Version;
use serde_json::Value;
use sqlx::{Database, Sqlite, SqlitePool};
use web3_address::ethereum::Address;

#[derive(Debug)]
pub struct PublisherRecord {
    /// Publisher primary key.
    pub publisher_id: i64,
    /// Address of the publisher.
    pub address: Address,
}

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
            self.publishers.iter().any(|a| a == address)
        }
    }
}

#[derive(Debug)]
pub struct PackageRecord {
    /// Namespace foreign key.
    pub namespace_id: i64,
    /// Package primary key.
    pub package_id: i64,
    /// Name of the package.
    pub name: String,
}

#[derive(Debug)]
pub struct VersionRecord {
    /// Publisher foreign key.
    pub publisher_id: i64,
    /// Package foreign key.
    pub package_id: i64,
    /// Version primary key.
    pub version_id: i64,
    /// Version of the package.
    pub version: Version,
    /// Package meta data.
    pub package: Value,
    /// Content identifier.
    pub content_id: Option<Cid>,
}

pub struct Package<T: Database> {
    marker: std::marker::PhantomData<T>,
}

impl Package<Sqlite> {
    /// Find a package by name.
    pub async fn find_by_name(
        pool: &SqlitePool,
        namespace_id: i64,
        name: &str,
    ) -> Result<Option<PackageRecord>> {
        let result = sqlx::query_as!(
            PackageRecord,
            r#"
                SELECT
                    namespace_id,
                    package_id,
                    name
                FROM packages
                WHERE namespace_id = ? AND name = ?
            "#,
            namespace_id,
            name
        )
        .fetch_optional(pool)
        .await?;
        Ok(result)
    }

    /// Find a package by name and version.
    pub async fn find_by_name_version(
        pool: &SqlitePool,
        namespace_id: i64,
        name: &str,
        version: &Version,
    ) -> Result<Option<VersionRecord>> {
        if let Some(package_record) =
            Package::<Sqlite>::find_by_name(pool, namespace_id, name).await?
        {
            let version = version.to_string();

            let result = sqlx::query!(
                r#"
                    SELECT
                        version_id,
                        publisher_id,
                        package_id,
                        version,
                        package,
                        content_id
                    FROM versions
                    WHERE package_id = ? AND version = ?
                "#,
                package_record.package_id,
                version,
            )
            .fetch_optional(pool)
            .await?;

            if let Some(record) = result {
                let version: Version = Version::parse(&record.version)?;
                let package: Value = serde_json::from_str(&record.package)?;

                let content_id = if let Some(cid) = record.content_id {
                    let cid: Cid = cid.try_into()?;
                    Some(cid)
                } else {
                    None
                };

                Ok(Some(VersionRecord {
                    publisher_id: record.publisher_id,
                    version_id: record.version_id,
                    package_id: record.package_id,
                    content_id,
                    version,
                    package,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Find or insert a new package.
    pub async fn find_or_insert(
        pool: &SqlitePool,
        namespace_id: i64,
        name: &str,
    ) -> Result<PackageRecord> {
        if let Some(record) =
            Package::<Sqlite>::find_by_name(pool, namespace_id, name).await?
        {
            Ok(record)
        } else {
            let mut conn = pool.acquire().await?;
            let id = sqlx::query!(
                r#"
                    INSERT INTO packages ( namespace_id, name, created_at )
                    VALUES ( ?1, ?2, datetime('now') )
                "#,
                namespace_id,
                name,
            )
            .execute(&mut conn)
            .await?
            .last_insert_rowid();
            Ok(PackageRecord {
                namespace_id,
                package_id: id,
                name: name.to_owned(),
            })
        }
    }

    /// Add a package version to a namespace.
    ///
    /// If a package already exists for the given name
    /// and version return `None`.
    pub async fn insert(
        pool: &SqlitePool,
        publisher: &Address,
        namespace: &str,
        name: &str,
        version: &Version,
        package: &Value,
        content_id: Option<&Cid>,
    ) -> Result<i64> {
        // Check the publisher exists
        let publisher_record =
            Publisher::<Sqlite>::find_by_address(pool, publisher)
                .await?
                .ok_or(Error::UnknownPublisher(*publisher))?;

        // Check the namespace exists
        let namespace_record =
            Namespace::<Sqlite>::find_by_name(pool, namespace)
                .await?
                .ok_or_else(|| {
                    Error::UnknownNamespace(namespace.to_string())
                })?;

        if !namespace_record.can_publish(publisher) {
            return Err(Error::Unauthorized(*publisher));
        }

        // Check the package / version does not already exist
        if Package::<Sqlite>::find_by_name_version(
            pool,
            namespace_record.namespace_id,
            name,
            version,
        )
        .await?
        .is_some()
        {
            return Err(Error::PackageExists(
                namespace_record.name.clone(),
                name.to_owned(),
                version.clone(),
            ));
        }

        // Find or insert the package
        let package = serde_json::to_string(package)?;
        let version = version.to_string();
        let package_record = Package::<Sqlite>::find_or_insert(
            pool,
            namespace_record.namespace_id,
            name,
        )
        .await?;

        let content_id = content_id.map(|cid| cid.to_string());

        // Insert the package version
        let mut conn = pool.acquire().await?;
        let id = sqlx::query!(
            r#"
                INSERT INTO versions ( publisher_id, package_id, version, package, content_id, created_at )
                VALUES ( ?1, ?2, ?3, ?4, ?5, datetime('now') )
            "#,
            publisher_record.publisher_id,
            package_record.package_id,
            version,
            package,
            content_id,
        )
        .execute(&mut conn)
        .await?
        .last_insert_rowid();

        Ok(id)
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
                INSERT INTO publishers ( address, created_at )
                VALUES ( ?1, datetime('now') )
            "#,
            addr,
        )
        .execute(&mut conn)
        .await?
        .last_insert_rowid();

        Ok(id)
    }

    /// Find a publisher by address.
    pub async fn find_by_address(
        pool: &SqlitePool,
        publisher: &Address,
    ) -> Result<Option<PublisherRecord>> {
        let addr = publisher.as_ref();

        let result = sqlx::query!(
            r#"
                SELECT
                    publisher_id,
                    address
                FROM publishers
                WHERE address = ?
            "#,
            addr
        )
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|r| PublisherRecord {
            publisher_id: r.publisher_id,
            address: *publisher,
        }))
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
                INSERT INTO namespaces ( name, publisher_id, created_at )
                VALUES ( ?1, ?2, datetime('now') )
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

    // TODO: allow removing a publisher from the namespace

    /// Find a namespace by name.
    pub async fn find_by_name(
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
