//! Model for publishers.
use sqlx::{sqlite::SqliteArguments, Arguments, QueryBuilder, SqlitePool};

use web3_address::ethereum::Address;

use crate::{value_objects::*, Error, Result};

/// Manage registry publishers.
pub struct PublisherModel;

impl PublisherModel {
    /// Insert a publisher.
    pub async fn insert(pool: &SqlitePool, owner: &Address) -> Result<i64> {
        let mut conn = pool.acquire().await?;

        let mut builder = QueryBuilder::new(
            r#"
                INSERT INTO publishers ( address, created_at )
                VALUES (
            "#,
        );
        let mut separated = builder.separated(", ");
        separated.push_bind(owner.as_ref());
        builder.push(", datetime('now') )");

        let id = builder
            .build()
            .execute(&mut conn)
            .await?
            .last_insert_rowid();

        Ok(id)
    }

    /// Insert a publisher and fetch the record.
    pub async fn insert_fetch(
        pool: &SqlitePool,
        owner: &Address,
    ) -> Result<PublisherRecord> {
        let id = PublisherModel::insert(pool, owner).await?;
        let record = PublisherModel::find_by_address(pool, owner)
            .await?
            .ok_or(Error::InsertFetch(id))?;
        Ok(record)
    }

    /// Find a publisher by address.
    pub async fn find_by_address(
        pool: &SqlitePool,
        publisher: &Address,
    ) -> Result<Option<PublisherRecord>> {
        let addr = publisher.as_ref();

        let mut args: SqliteArguments = Default::default();
        args.add(addr);

        let record = sqlx::query_as_with::<_, PublisherRecord, _>(
            r#"
                SELECT
                    publisher_id,
                    address,
                    created_at
                FROM publishers
                WHERE address = ?
            "#,
            args,
        )
        .fetch_optional(pool)
        .await?;

        Ok(record)
    }
}
