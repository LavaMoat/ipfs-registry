use cid::Cid;
use semver::Version;
use serde_json::Value;
use sqlx::{Database, Sqlite, SqlitePool};
use web3_address::ethereum::Address;
use time::OffsetDateTime;

use crate::{value_objects::*, Error, Result};

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
        let record = sqlx::query!(
            r#"
                SELECT
                    namespace_id,
                    package_id,
                    created_at,
                    name
                FROM packages
                WHERE namespace_id = ? AND name = ?
            "#,
            namespace_id,
            name
        )
        .fetch_optional(pool)
        .await?;

        let record = if let Some(record) = record {
            let created_at = parse_date_time(&record.created_at)?;

            Some(PackageRecord {
                namespace_id: record.namespace_id,
                package_id: record.package_id,
                name: record.name,
                created_at
            })
        } else { None };

        Ok(record)
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
                        content_id,
                        created_at
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

                // Parse to time type
                let created_at = parse_date_time(&record.created_at)?;

                Ok(Some(VersionRecord {
                    publisher_id: record.publisher_id,
                    version_id: record.version_id,
                    package_id: record.package_id,
                    content_id,
                    version,
                    package,
                    created_at,
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
                // WARN: may not exactly match the database value
                created_at: OffsetDateTime::now_utc(),
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

        let record = sqlx::query!(
            r#"
                SELECT
                    publisher_id,
                    created_at
                FROM publishers
                WHERE address = ?
            "#,
            addr
        )
        .fetch_optional(pool)
        .await?;

        let record = if let Some(record) = record {
            let created_at = parse_date_time(&record.created_at)?;
            Some(PublisherRecord {
                publisher_id: record.publisher_id,
                address: *publisher,
                created_at,
            })
        } else { None };

        Ok(record)
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
                    namespaces.created_at,
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

            // Parse to time type
            let created_at = parse_date_time(&result.created_at)?;

            Ok(Some(NamespaceRecord {
                namespace_id: result.namespace_id,
                name: result.name,
                owner,
                publishers,
                created_at,
            }))
        } else {
            Ok(None)
        }
    }
}
