use anyhow::Result;
use serial_test::serial;
use std::path::PathBuf;

use crate::test_utils::*;
use semver::Version;

use sqlx::sqlite::SqlitePool;

#[tokio::test]
#[serial]
async fn integration_database() -> Result<()> {
    println!("database integration tests...");

    let url = "sqlite::memory:";

    let pool = SqlitePool::connect(url).await?;
    sqlx::migrate!().run(&pool).await?;

    Ok(())
}
