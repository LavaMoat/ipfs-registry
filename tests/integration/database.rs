use anyhow::Result;
use serial_test::serial;

use crate::test_utils::*;

use semver::Version;
use sqlx::SqlitePool;

use ipfs_registry_core::{Namespace, PackageName};
use ipfs_registry_database::{
    Error, NamespaceModel, PackageModel, PublisherModel,
};

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
    let publisher_id = PublisherModel::insert(&pool, &address).await?;

    let user_publisher_id =
        PublisherModel::insert(&pool, &authorized_address).await?;

    let _unauthorized_publisher_id =
        PublisherModel::insert(&pool, &unauthorized_address).await?;

    let namespace = Namespace::new_unchecked("mock-namespace");

    // Create a namespace
    let namespace_id =
        NamespaceModel::insert(&pool, &namespace, publisher_id).await?;

    assert!(namespace_id > 0);

    // Add another publisher to the namespace
    NamespaceModel::add_publisher(&pool, namespace_id, user_publisher_id)
        .await?;

    let ns = NamespaceModel::find_by_name(&pool, &namespace).await?;

    assert!(ns.is_some());
    let ns = ns.unwrap();

    assert!(ns.can_publish(&address));
    assert!(ns.can_publish(&authorized_address));
    assert!(ns.can_publish(&unauthorized_address) == false);

    assert_eq!(address, ns.owner);
    assert_eq!(&authorized_address, ns.publishers.get(0).unwrap());

    let pointer = mock_pointer(None)?;

    let mock_package = PackageName::new_unchecked("mock-package");
    let mock_version = Version::new(1, 0, 0);

    // Verify for publishing
    let (publisher_record, namespace_record) =
        PackageModel::verify_publish(&pool, &address, &namespace).await?;

    // Publish as the namespace owner
    let result = PackageModel::insert(
        &pool,
        &publisher_record,
        &namespace_record,
        &address,
        &pointer,
    )
    .await?;
    assert!(result > 0);

    // Publish as an authorized publisher
    let result = PackageModel::insert(
        &pool,
        &publisher_record,
        &namespace_record,
        &authorized_address,
        &mock_pointer(Some(Version::new(1, 0, 1)))?,
    )
    .await?;
    assert!(result > 0);

    // Attempt to publish an existing version - `Err`
    let result = PackageModel::assert_publish_safe(
        &pool,
        &namespace_record,
        &mock_package,
        &mock_version,
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
    let result =
        PackageModel::verify_publish(&pool, &unknown_address, &namespace)
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
    let result = PackageModel::verify_publish(
        &pool,
        &unauthorized_address,
        &namespace,
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
    let result = PackageModel::verify_publish(
        &pool,
        &address,
        &Namespace::new_unchecked("unknown-namespace"),
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

    // Check we can get the published package / version
    let package_record = PackageModel::find_by_name_version(
        &pool,
        namespace_id,
        &mock_package,
        &mock_version,
    )
    .await?;

    assert!(package_record.is_some());
    let package_record = package_record.unwrap();

    assert!(package_record.publisher_id > 0);
    assert!(package_record.package_id > 0);
    assert!(package_record.version_id > 0);
    assert_eq!(&package_record.version, &mock_version);
    assert_eq!(&package_record.package, &pointer.package);

    let versions = PackageModel::list_versions(
        &pool,
        &namespace,
        &mock_package,
        Default::default(),
    )
    .await?;

    assert_eq!(2, versions.len());

    let packages =
        PackageModel::list_packages(&pool, &namespace, Default::default())
            .await?;

    assert!(packages.len() > 0);

    let package = packages.get(0).unwrap();
    // Listing packages includes the latest version for each package
    assert!(package.versions.len() == 1);

    let version = package.versions.get(0).unwrap();
    // Check it is actually the most recent version -
    // two packages were published above ^^^
    assert_eq!(&Version::new(1, 0, 1), &version.version);

    Ok(())
}
