use anyhow::Result;
use serial_test::serial;

//use crate::test_utils::*;

use web3_address::ethereum::Address;
use sqlx::{sqlite::SqlitePool, Sqlite};

use ipfs_registry_database::Namespace;

#[tokio::test]
#[serial]
async fn integration_database() -> Result<()> {
    let url = "sqlite::memory:";

    let pool = SqlitePool::connect(url).await?;
    sqlx::migrate!().run(&pool).await?;

    let address: Address = "0x1fc770ac21067a04f83101ebf19a670db9e3eb21".parse()?;

    let id = Namespace::<Sqlite>::add(
        &pool, "mock-namespace", &address).await?;

    assert!(id > 0);

    let row = Namespace::<Sqlite>::get_by_id(&pool, id).await?;

    assert!(row.is_some());

    Ok(())
}
