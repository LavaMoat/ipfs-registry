use semver::Version;

use sqlx::{sqlite::SqliteArguments, Arguments, QueryBuilder, SqlitePool};
use time::OffsetDateTime;
use web3_address::ethereum::Address;

use crate::{value_objects::*, Error, Result};
use ipfs_registry_core::{Namespace, PackageKey, Pointer};

use super::{NamespaceModel, Pager, PublisherModel};

pub struct PackageModel;

impl PackageModel {
    /// List packages for a namespace.
    pub async fn list_packages(
        pool: &SqlitePool,
        namespace: &Namespace,
        pager: Pager,
    ) -> Result<Vec<PackageRecord>> {
        // Check the namespace exists
        let namespace_record = NamespaceModel::find_by_name(pool, namespace)
            .await?
            .ok_or_else(|| Error::UnknownNamespace(namespace.clone()))?;

        let mut args: SqliteArguments = Default::default();
        args.add(namespace_record.namespace_id);
        args.add(pager.limit);
        args.add(pager.offset);

        let sql = format!(
            r#"
            SELECT
                namespace_id,
                package_id,
                created_at,
                name
            FROM packages
            WHERE namespace_id = ?
            ORDER BY name {}
            LIMIT ? OFFSET ?
        "#,
            pager.direction.as_str()
        );

        let records = sqlx::query_as_with::<_, PackageRecord, _>(&sql, args)
            .fetch_all(pool)
            .await?;

        Ok(records)
    }

    /// Find a package version by package key.
    pub async fn find_by_key(
        pool: &SqlitePool,
        package_key: &PackageKey,
    ) -> Result<Option<VersionRecord>> {
        match package_key {
            PackageKey::Pointer(namespace, name, version) => {
                let namespace_record =
                    NamespaceModel::find_by_name(pool, namespace)
                        .await?
                        .ok_or_else(|| {
                            Error::UnknownNamespace(namespace.clone())
                        })?;
                PackageModel::find_by_name_version(
                    pool,
                    namespace_record.namespace_id,
                    name,
                    version,
                )
                .await
            }
            PackageKey::Cid(cid) => {
                let mut args: SqliteArguments = Default::default();
                args.add(cid.to_string());

                let record = sqlx::query_as_with::<_, VersionRecord, _>(
                    r#"SELECT * FROM versions WHERE content_id = ?"#,
                    args,
                )
                .fetch_optional(pool)
                .await?;

                Ok(record)
            }
        }
    }

    /// Find a package by name.
    pub async fn find_by_name(
        pool: &SqlitePool,
        namespace_id: i64,
        name: &str,
    ) -> Result<Option<PackageRecord>> {
        let mut args: SqliteArguments = Default::default();
        args.add(namespace_id);
        args.add(name);

        let record = sqlx::query_as_with::<_, PackageRecord, _>(
            r#"
                SELECT
                    namespace_id,
                    package_id,
                    created_at,
                    name
                FROM packages
                WHERE namespace_id = ? AND name = ?
            "#,
            args,
        )
        .fetch_optional(pool)
        .await?;

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
            PackageModel::find_by_name(pool, namespace_id, name).await?
        {
            let mut args: SqliteArguments = Default::default();
            args.add(package_record.package_id);
            args.add(version.major as i64);
            args.add(version.minor as i64);
            args.add(version.patch as i64);
            args.add(version.pre.to_string());
            args.add(version.build.to_string());

            let record = sqlx::query_as_with::<_, VersionRecord, _>(
                r#"
                    SELECT * FROM versions
                    WHERE package_id = ? AND major = ? AND minor = ? AND patch = ? AND pre = ? AND build = ?
                "#,
                args
            )
            .fetch_optional(pool)
            .await?;

            Ok(record)
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
            PackageModel::find_by_name(pool, namespace_id, name).await?
        {
            Ok(record)
        } else {
            let mut conn = pool.acquire().await?;

            let mut builder = QueryBuilder::new(
                r#"
                    INSERT INTO packages ( namespace_id, name, created_at )
                    VALUES (
                "#,
            );
            let mut separated = builder.separated(", ");
            separated.push_bind(namespace_id);
            separated.push_bind(name);
            builder.push(", datetime('now') )");

            let id = builder
                .build()
                .execute(&mut conn)
                .await?
                .last_insert_rowid();

            // FIXME: fetch from the database!

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
        publisher_record: &PublisherRecord,
        namespace_record: &NamespaceRecord,
        _publisher: &Address,
        pointer: &Pointer,
    ) -> Result<i64> {
        let name = &pointer.definition.artifact.package.name;
        let version = &pointer.definition.artifact.package.version;

        // Find or insert the package
        let package = serde_json::to_string(&pointer.package)?;

        //let version = version.to_string();
        let package_record = PackageModel::find_or_insert(
            pool,
            namespace_record.namespace_id,
            name,
        )
        .await?;

        // Insert the package version
        let mut conn = pool.acquire().await?;

        let mut builder = QueryBuilder::new(
            r#"
                INSERT INTO versions ( publisher_id, package_id, major, minor, patch, pre, build, package, content_id, signature, checksum, created_at )
                VALUES (
            "#,
        );
        let mut separated = builder.separated(", ");
        separated.push_bind(publisher_record.publisher_id);
        separated.push_bind(package_record.package_id);
        separated.push_bind(version.major as i64);
        separated.push_bind(version.minor as i64);
        separated.push_bind(version.patch as i64);
        separated.push_bind(version.pre.to_string());
        separated.push_bind(version.build.to_string());
        separated.push_bind(package);
        separated.push_bind(pointer.definition.object.to_string());
        separated.push_bind(pointer.definition.signature.value.to_vec());
        separated.push_bind(pointer.definition.checksum.to_vec());
        builder.push(", datetime('now') )");

        let id = builder
            .build()
            .execute(&mut conn)
            .await?
            .last_insert_rowid();

        Ok(id)
    }

    /// Assert publishing is ok by checking a package
    /// with the given name and version does not already exist.
    pub async fn assert_publish_safe(
        pool: &SqlitePool,
        namespace_record: &NamespaceRecord,
        name: &str,
        version: &Version,
    ) -> Result<()> {
        // Check the package / version does not already exist
        if PackageModel::find_by_name_version(
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
        Ok(())
    }

    /// Verify the publisher and namespace before publishing.
    pub async fn verify_publish(
        pool: &SqlitePool,
        publisher: &Address,
        namespace: &Namespace,
    ) -> Result<(PublisherRecord, NamespaceRecord)> {
        // Check the publisher exists
        let publisher_record =
            PublisherModel::find_by_address(pool, publisher)
                .await?
                .ok_or(Error::UnknownPublisher(*publisher))?;

        // Check the namespace exists
        let namespace_record = NamespaceModel::find_by_name(pool, namespace)
            .await?
            .ok_or_else(|| Error::UnknownNamespace(namespace.clone()))?;

        if !namespace_record.can_publish(publisher) {
            return Err(Error::Unauthorized(*publisher));
        }

        Ok((publisher_record, namespace_record))
    }
}