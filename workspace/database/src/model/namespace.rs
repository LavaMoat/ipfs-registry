use sqlx::{sqlite::SqliteArguments, Arguments, QueryBuilder, SqlitePool};

use crate::{value_objects::*, Error, Result};
use ipfs_registry_core::Namespace;

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
                INSERT INTO namespaces ( name, publisher_id, created_at )
                VALUES (
            "#,
        );
        let mut separated = builder.separated(", ");
        separated.push_bind(name.as_str());
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

    /// Add a publisher to a namespace.
    pub async fn add_publisher(
        pool: &SqlitePool,
        namespace_id: i64,
        publisher_id: i64,
    ) -> Result<i64> {
        let mut conn = pool.acquire().await?;
        let mut builder = QueryBuilder::new(
            r#"
                INSERT INTO namespace_publishers ( namespace_id, publisher_id )
                VALUES (
            "#,
        );
        let mut separated = builder.separated(", ");
        separated.push_bind(namespace_id);
        separated.push_bind(publisher_id);
        builder.push(" )");

        let id = builder
            .build()
            .execute(&mut conn)
            .await?
            .last_insert_rowid();

        Ok(id)
    }

    // TODO: allow removing a publisher from the namespace

    /// Find a namespace by name.
    pub async fn find_by_name(
        pool: &SqlitePool,
        name: &Namespace,
    ) -> Result<Option<NamespaceRecord>> {
        let ns = name.as_str();
        let mut args: SqliteArguments = Default::default();
        args.add(ns);

        let record = sqlx::query_as_with::<_, NamespaceRecord, _>(
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
                        publishers.address
                    FROM namespace_publishers
                    INNER JOIN publishers
                    ON (namespace_publishers.publisher_id = publishers.publisher_id)
                    WHERE namespace_id = ?
                "#,
                args
            )
            .fetch_all(pool)
            .await?;

            for user in users {
                record.publishers.push(user.address);
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
