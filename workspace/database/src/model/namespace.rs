use crate::{value_objects::*, Error, Result};
use sqlx::{sqlite::SqliteArguments, Arguments, QueryBuilder, SqlitePool};
use web3_address::ethereum::Address;

use ipfs_registry_core::{confusable_skeleton, Namespace, PackageName};

use crate::model::{PackageModel, PublisherModel};

pub struct NamespaceModel;

impl NamespaceModel {
    /// Add a namespace.
    pub async fn insert(
        pool: &SqlitePool,
        name: &Namespace,
        publisher_id: i64,
    ) -> Result<i64> {
        let mut conn = pool.acquire().await?;

        let mut builder = QueryBuilder::new(
            r#"
                INSERT INTO namespaces ( name, skeleton, publisher_id, created_at )
                VALUES (
            "#,
        );
        let skeleton = confusable_skeleton(name.as_str());
        let mut separated = builder.separated(", ");
        separated.push_bind(name.as_str());
        separated.push_bind(&skeleton);
        separated.push_bind(publisher_id);
        builder.push(", datetime('now') )");

        let id = builder
            .build()
            .execute(&mut conn)
            .await?
            .last_insert_rowid();

        Ok(id)
    }

    /// Insert a namespace and fetch the record.
    pub async fn insert_fetch(
        pool: &SqlitePool,
        name: &Namespace,
        publisher_id: i64,
    ) -> Result<NamespaceRecord> {
        let id = NamespaceModel::insert(pool, name, publisher_id).await?;
        let record = NamespaceModel::find_by_name(pool, name)
            .await?
            .ok_or(Error::InsertFetch(id))?;
        Ok(record)
    }

    /// Verify an address can access a namespace.
    ///
    /// Further access control checks may be required depending
    /// upon the operation.
    pub async fn can_access_namespace(
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

        if !namespace_record.has_user(publisher) {
            return Err(Error::Unauthorized(*publisher));
        }

        Ok((publisher_record, namespace_record))
    }

    /// Add a user to this namespace.
    pub async fn add_user(
        pool: &SqlitePool,
        namespace: &Namespace,
        caller: &Address,
        user: &Address,
        administrator: bool,
        restrictions: Vec<&PackageName>,
    ) -> Result<i64> {
        let (_, namespace_record) =
            NamespaceModel::can_access_namespace(pool, caller, namespace)
                .await?;

        // Only the owner can add administrators
        if administrator && !namespace_record.is_owner(caller) {
            return Err(Error::Unauthorized(*caller));
        }

        // Only administrators can add users
        if !namespace_record.can_administrate(caller) {
            return Err(Error::Unauthorized(*caller));
        }

        if namespace_record.find_user(&user).is_some() {
            return Err(Error::UserExists(
                *user,
                namespace_record.name.to_string(),
            ));
        }

        // User must already be registered
        let user_record = PublisherModel::find_by_address(pool, user)
            .await?
            .ok_or(Error::UnknownPublisher(*user))?;

        let packages = PackageModel::find_many_by_name(
            pool,
            namespace_record.namespace_id,
            restrictions,
        )
        .await?;

        let mut restrictions = Vec::new();
        for (name, pkg) in packages {
            let pkg = pkg.ok_or(Error::UnknownPackage(name.to_string()))?;
            restrictions.push(pkg.package_id);
        }

        NamespaceModel::add_publisher(
            pool,
            namespace_record.namespace_id,
            user_record.publisher_id,
            administrator,
            restrictions,
        )
        .await
    }

    /// Add a publisher to a namespace.
    async fn add_publisher(
        pool: &SqlitePool,
        namespace_id: i64,
        publisher_id: i64,
        administrator: bool,
        restrictions: Vec<i64>,
    ) -> Result<i64> {
        let administrator = if administrator { 1 } else { 0 };
        let mut tx = pool.begin().await?;
        let mut builder = QueryBuilder::new(
            r#"
                INSERT INTO namespace_publishers
                    ( namespace_id, publisher_id, administrator )
                VALUES (
            "#,
        );
        let mut separated = builder.separated(", ");
        separated.push_bind(namespace_id);
        separated.push_bind(publisher_id);
        separated.push_bind(administrator);
        builder.push(" )");

        let id = builder.build().execute(&mut tx).await?.last_insert_rowid();

        for package_id in restrictions {
            let mut builder = QueryBuilder::new(
                r#"
                    INSERT INTO publisher_restrictions
                        ( publisher_id, package_id )
                    VALUES (
                "#,
            );
            let mut separated = builder.separated(", ");
            separated.push_bind(publisher_id);
            separated.push_bind(package_id);
            builder.push(" )");

            builder.build().execute(&mut tx).await?;
        }

        tx.commit().await?;

        Ok(id)
    }

    // TODO: allow removing a publisher from the namespace

    /// Find a namespace by name.
    pub async fn find_by_name(
        pool: &SqlitePool,
        name: &Namespace,
    ) -> Result<Option<NamespaceRecord>> {
        let skeleton = confusable_skeleton(name.as_str());
        let mut args: SqliteArguments = Default::default();
        args.add(skeleton);

        let record = sqlx::query_as_with::<_, NamespaceRecord, _>(
            r#"
                SELECT
                    namespaces.namespace_id,
                    namespaces.name,
                    namespaces.publisher_id,
                    namespaces.created_at,
                    publishers.address
                FROM namespaces
                LEFT JOIN publishers
                ON (namespaces.publisher_id = publishers.publisher_id)
                WHERE skeleton = ?
            "#,
            args,
        )
        .fetch_optional(pool)
        .await?;

        if let Some(mut record) = record {
            let mut args: SqliteArguments = Default::default();
            args.add(record.namespace_id);

            let users = sqlx::query_as_with::<_, UserRecord, _>(
                r#"
                    SELECT
                        namespace_publishers.namespace_id,
                        namespace_publishers.publisher_id,
                        namespace_publishers.administrator,
                        publishers.address,
                        GROUP_CONCAT(publisher_restrictions.package_id) as package_ids
                    FROM namespace_publishers
                    LEFT JOIN publishers
                        ON (namespace_publishers.publisher_id = publishers.publisher_id)
                    LEFT JOIN publisher_restrictions
                        ON (namespace_publishers.publisher_id = publisher_restrictions.publisher_id)
                    WHERE namespace_id = ?
                    GROUP BY namespace_publishers.publisher_id
                "#,
                args
            )
            .fetch_all(pool)
            .await?;

            for user in users {
                record.publishers.push(user);
            }

            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    /// Find a namespace by id.
    pub async fn find_namespace_by_id(
        pool: &SqlitePool,
        namespace_id: i64,
    ) -> Result<Option<NamespaceRecord>> {
        let mut args: SqliteArguments = Default::default();
        args.add(namespace_id);
        let record = sqlx::query_as_with::<_, NamespaceRecord, _>(
            r#"SELECT * FROM namespaces WHERE namespace_id = ?"#,
            args,
        )
        .fetch_optional(pool)
        .await?;
        Ok(record)
    }
}
