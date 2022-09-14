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
async fn integration_access_control() -> Result<()> {
    let url = "sqlite::memory:";
    let pool = SqlitePool::connect(url).await?;
    sqlx::migrate!().run(&pool).await?;

    let (_, address) = new_signing_key();
    let (_, authorized_address) = new_signing_key();
    let (_, restricted_address) = new_signing_key();
    let (_, administrator_address) = new_signing_key();
    let (_, delegated_address) = new_signing_key();
    let (_, unauthorized_address) = new_signing_key();

    // Create a publisher to own the namespace
    let publisher_id = PublisherModel::insert(&pool, &address).await?;

    // Register the addresses
    PublisherModel::insert(&pool, &authorized_address).await?;
    PublisherModel::insert(&pool, &restricted_address).await?;
    PublisherModel::insert(&pool, &administrator_address).await?;
    PublisherModel::insert(&pool, &delegated_address).await?;
    PublisherModel::insert(&pool, &unauthorized_address).await?;

    let namespace = Namespace::new_unchecked("mock-namespace");

    // Create a namespace
    let namespace_id =
        NamespaceModel::insert(&pool, &namespace, publisher_id).await?;

    assert!(namespace_id > 0);

    // Add a user to the namespace with no restrictions
    NamespaceModel::add_user(
        &pool,
        &namespace,
        &address,
        &authorized_address,
        false,
        vec![],
    )
    .await?;

    // Add an administrator to the namespace
    NamespaceModel::add_user(
        &pool,
        &namespace,
        &address,
        &administrator_address,
        true,
        vec![],
    )
    .await?;

    // Administrator can add other users
    assert!(NamespaceModel::add_user(
        &pool,
        &namespace,
        &administrator_address,
        &delegated_address,
        false,
        vec![],
    )
    .await
    .is_ok());

    let mut pointer = mock_pointer(None)?;

    let mock_package = PackageName::new_unchecked("mock-package");
    let alt_package = PackageName::new_unchecked("alt-package");
    let private_package = PackageName::new_unchecked("private-package");
    let mock_version = Version::new(1, 0, 0);

    // Verify for publishing
    let (publisher_record, namespace_record) =
        NamespaceModel::can_access_namespace(&pool, &address, &namespace)
            .await?;

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

    // Publish another package
    pointer.definition.artifact.package.name = alt_package.clone();
    let result = PackageModel::insert(
        &pool,
        &publisher_record,
        &namespace_record,
        &address,
        &pointer,
    )
    .await?;
    assert!(result > 0);

    pointer.definition.artifact.package.name = private_package.clone();
    let result = PackageModel::insert(
        &pool,
        &publisher_record,
        &namespace_record,
        &address,
        &pointer,
    )
    .await?;
    assert!(result > 0);

    // Get the package records so we can get the id to create a restriction
    let (_, package_record) = PackageModel::find_by_name_version(
        &pool,
        namespace_id,
        &mock_package,
        &mock_version,
    )
    .await?;

    assert!(package_record.is_some());
    let package_record = package_record.unwrap();

    let (_, alt_record) = PackageModel::find_by_name_version(
        &pool,
        namespace_id,
        &alt_package,
        &mock_version,
    )
    .await?;

    assert!(alt_record.is_some());
    let alt_record = alt_record.unwrap();

    let package_restrictions =
        vec![package_record.package_id, alt_record.package_id];

    // Add a restricted user to the namespace
    NamespaceModel::add_user(
        &pool,
        &namespace,
        &address,
        &restricted_address,
        false,
        vec![&mock_package, &alt_package],
    )
    .await?;

    let ns = NamespaceModel::find_by_name(&pool, &namespace).await?;

    assert!(ns.is_some());
    let ns = ns.unwrap();

    //println!("{:#?}", ns);

    assert_eq!(4, ns.publishers.len());
    assert!(ns.has_user(&address));
    assert!(ns.has_user(&authorized_address));
    assert!(ns.has_user(&administrator_address));
    assert!(ns.has_user(&delegated_address));
    assert!(ns.has_user(&restricted_address));
    assert!(ns.has_user(&unauthorized_address) == false);

    assert_eq!(address, ns.owner);
    assert_eq!(&authorized_address, &ns.publishers.get(0).unwrap().address);

    let restricted_user = ns
        .publishers
        .iter()
        .find(|u| &u.address == &restricted_address);

    assert!(restricted_user.is_some());
    let restricted_user = restricted_user.unwrap();

    assert_eq!(&package_restrictions, &restricted_user.restrictions);

    // The namespace owner can publish a newer version
    assert!(PackageModel::can_publish_package(
        &pool,
        &address,
        &ns,
        &mock_package,
        &Version::new(2, 0, 0),
    )
    .await
    .is_ok());

    // An authorized unrestricted user can also publish
    assert!(PackageModel::can_publish_package(
        &pool,
        &authorized_address,
        &ns,
        &mock_package,
        &Version::new(2, 0, 0),
    )
    .await
    .is_ok());

    // Administrator can publish to the private package
    assert!(PackageModel::can_publish_package(
        &pool,
        &administrator_address,
        &ns,
        &private_package,
        &Version::new(2, 0, 0),
    )
    .await
    .is_ok());

    // Restricted user can also publish as it has access to this package
    assert!(PackageModel::can_publish_package(
        &pool,
        &restricted_address,
        &ns,
        &mock_package,
        &Version::new(2, 0, 0),
    )
    .await
    .is_ok());

    // Unauthorized address is denied
    let result = PackageModel::can_publish_package(
        &pool,
        &unauthorized_address,
        &ns,
        &mock_package,
        &Version::new(2, 0, 0),
    )
    .await;
    let is_unauthorized = if let Err(Error::Unauthorized(_)) = result {
        true
    } else {
        false
    };
    assert!(is_unauthorized);

    // Restricted user has not been granted access to the private package
    let result = PackageModel::can_publish_package(
        &pool,
        &restricted_address,
        &ns,
        &private_package,
        &Version::new(2, 0, 0),
    )
    .await;
    let is_unauthorized = if let Err(Error::Unauthorized(_)) = result {
        true
    } else {
        false
    };
    assert!(is_unauthorized);

    // Restricted user cannot add a user as it is not an administrator
    let result = NamespaceModel::add_user(
        &pool,
        &namespace,
        &restricted_address,
        &unauthorized_address,
        false,
        vec![],
    )
    .await;
    let is_unauthorized = if let Err(Error::Unauthorized(_)) = result {
        true
    } else {
        false
    };
    assert!(is_unauthorized);

    // Administrator cannot add other administrators
    let result = NamespaceModel::add_user(
        &pool,
        &namespace,
        &administrator_address,
        &unauthorized_address,
        true,
        vec![],
    )
    .await;
    let is_unauthorized = if let Err(Error::Unauthorized(_)) = result {
        true
    } else {
        false
    };
    assert!(is_unauthorized);

    Ok(())
}
