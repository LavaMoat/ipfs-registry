use anyhow::Result;
use serial_test::serial;

use crate::test_utils::*;

use cid::Cid;
use semver::Version;
use serde_json::Value;
use sqlx::{Sqlite, SqlitePool};

use ipfs_registry_database::{Error, Namespace, Package, Publisher};

#[tokio::test]
#[serial]
async fn integration_database() -> Result<()> {
    let url = "sqlite::memory:";

    let pool = SqlitePool::connect(url).await?;
    sqlx::migrate!().run(&pool).await?;

    let (_, address) = new_signing_key();
    let (_, authorized_address) = new_signing_key();
    let (_, unknown_address) = new_signing_key();
    let (_, unauthorized_address) = new_signing_key();

    // Create a publisher to own the namespace
    let publisher_id = Publisher::<Sqlite>::add(&pool, &address).await?;

    let user_publisher_id =
        Publisher::<Sqlite>::add(&pool, &authorized_address).await?;

    let _unauthorized_publisher_id =
        Publisher::<Sqlite>::add(&pool, &unauthorized_address).await?;

    let namespace = "mock-namespace";

    // Create a namespace
    let namespace_id =
        Namespace::<Sqlite>::add(&pool, namespace, publisher_id).await?;

    assert!(namespace_id > 0);

    // Add another publisher to the namespace
    Namespace::<Sqlite>::add_publisher(
        &pool,
        namespace_id,
        user_publisher_id,
    )
    .await?;

    let ns = Namespace::<Sqlite>::find_by_name(&pool, namespace).await?;

    assert!(ns.is_some());
    let ns = ns.unwrap();

    assert!(ns.can_publish(&address));
    assert!(ns.can_publish(&authorized_address));
    assert!(ns.can_publish(&unauthorized_address) == false);

    assert_eq!(address, ns.owner);
    assert_eq!(&authorized_address, ns.publishers.get(0).unwrap());

    let mock_package = "mock-package";
    let mock_version = Version::new(1, 0, 0);
    let mock_value = Value::Null;
    let cid: Cid =
        "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"
            .try_into()?;
    let mock_content_id = Some(&cid);

    // Publish as the namespace owner
    let result = Package::<Sqlite>::insert(
        &pool,
        &address,
        namespace,
        mock_package,
        &mock_version,
        &mock_value,
        mock_content_id,
    )
    .await?;
    assert!(result > 0);

    // Publish as an authorized publisher
    let result = Package::<Sqlite>::insert(
        &pool,
        &authorized_address,
        namespace,
        mock_package,
        &Version::new(1, 0, 1),
        &mock_value,
        mock_content_id,
    )
    .await?;
    assert!(result > 0);

    // Attempt to publish an existing version - `Err`
    let result = Package::<Sqlite>::insert(
        &pool,
        &address,
        namespace,
        mock_package,
        &mock_version,
        &mock_value,
        mock_content_id,
    )
    .await;
    assert!(result.is_err());

    let is_package_exists = if let Err(Error::PackageExists(_, _, _)) = result
    {
        true
    } else {
        false
    };
    assert!(is_package_exists);

    // Publish using an address that is not registered - `Err`
    let result = Package::<Sqlite>::insert(
        &pool,
        &unknown_address,
        namespace,
        mock_package,
        &mock_version,
        &mock_value,
        mock_content_id,
    )
    .await;
    assert!(result.is_err());

    let is_unknown_publisher = if let Err(Error::UnknownPublisher(_)) = result
    {
        true
    } else {
        false
    };
    assert!(is_unknown_publisher);

    // Publish using an address that is not authorized - `Err`
    let result = Package::<Sqlite>::insert(
        &pool,
        &unauthorized_address,
        namespace,
        mock_package,
        &mock_version,
        &mock_value,
        mock_content_id,
    )
    .await;
    assert!(result.is_err());

    let is_unauthorized = if let Err(Error::Unauthorized(_)) = result {
        true
    } else {
        false
    };
    assert!(is_unauthorized);

    // Publish using a namespace that does not exist - `Err`
    let result = Package::<Sqlite>::insert(
        &pool,
        &address,
        "unknown-namespace",
        mock_package,
        &mock_version,
        &mock_value,
        mock_content_id,
    )
    .await;
    assert!(result.is_err());

    let is_unknown_namespace = if let Err(Error::UnknownNamespace(_)) = result
    {
        true
    } else {
        false
    };
    assert!(is_unknown_namespace);

    Ok(())
}
