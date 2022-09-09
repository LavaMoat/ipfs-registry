use anyhow::Result;
use serial_test::serial;

use crate::test_utils::*;

use sqlx::{Sqlite, SqlitePool};

use ipfs_registry_database::{Namespace, Publisher};

#[tokio::test]
#[serial]
async fn integration_database() -> Result<()> {
    let url = "sqlite::memory:";

    let pool = SqlitePool::connect(url).await?;
    sqlx::migrate!().run(&pool).await?;

    let (_, address) = new_signing_key();
    let (_, user_address) = new_signing_key();
    let (_, unauthorized_address) = new_signing_key();

    // Create a publisher to own the namespace
    let publisher_id = Publisher::<Sqlite>::add(&pool, &address).await?;

    let user_publisher_id =
        Publisher::<Sqlite>::add(&pool, &user_address).await?;

    // Create a namespace
    let namespace_id =
        Namespace::<Sqlite>::add(&pool, "mock-namespace", publisher_id)
            .await?;

    assert!(namespace_id > 0);

    // Add another publisher to the namespace
    Namespace::<Sqlite>::add_publisher(
        &pool,
        namespace_id,
        user_publisher_id,
    )
    .await?;

    let ns =
        Namespace::<Sqlite>::find_by_name(&pool, "mock-namespace").await?;

    assert!(ns.is_some());
    let ns = ns.unwrap();

    assert!(ns.can_publish(&address));
    assert!(ns.can_publish(&user_address));
    assert!(ns.can_publish(&unauthorized_address) == false);

    assert_eq!(address, ns.owner);
    assert_eq!(&user_address, ns.publishers.get(0).unwrap());

    Ok(())
}
