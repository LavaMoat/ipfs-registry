use anyhow::Result;
use serial_test::serial;

use crate::test_utils::*;

use semver::{Version, VersionReq};
use sqlx::SqlitePool;

use ipfs_registry_core::{Namespace, PackageName};
use ipfs_registry_database::{NamespaceModel, PackageModel, PublisherModel};

#[tokio::test]
#[serial]
async fn integration_semver() -> Result<()> {
    let url = "sqlite::memory:";
    let pool = SqlitePool::connect(url).await?;
    sqlx::migrate!().run(&pool).await?;

    let (_, address) = new_signing_key();

    // Create a publisher to own the namespace
    let publisher_id = PublisherModel::insert(&pool, &address).await?;

    // Create a namespace
    let namespace = Namespace::new_unchecked("mock-namespace");
    let _namespace_id =
        NamespaceModel::insert(&pool, &namespace, publisher_id).await?;

    let pointer = mock_pointer(None)?;

    let mock_namespace = Namespace::new_unchecked("mock-namespace");
    let mock_package = PackageName::new_unchecked("mock-package");

    // Verify for publishing
    let (publisher_record, namespace_record) =
        PackageModel::verify_publish(&pool, &address, &namespace).await?;

    // Publish 1.0.0
    let result = PackageModel::insert(
        &pool,
        &publisher_record,
        &namespace_record,
        &address,
        &pointer,
    )
    .await?;
    assert!(result > 0);

    // Publish 1.0.1
    let result = PackageModel::insert(
        &pool,
        &publisher_record,
        &namespace_record,
        &address,
        &mock_pointer(Some(Version::new(1, 0, 1)))?,
    )
    .await?;
    assert!(result > 0);

    let request = VersionReq::parse("=1.0.0")?;
    let mut versions = PackageModel::find_versions(
        &pool,
        &mock_namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    assert!(versions.len() == 1);
    assert_eq!(Version::new(1, 0, 0), versions.remove(0).version);

    let request = VersionReq::parse("=1.0.0, =1.0.1")?;
    let mut versions = PackageModel::find_versions(
        &pool,
        &mock_namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    assert!(versions.len() == 2);
    assert_eq!(Version::new(1, 0, 0), versions.remove(0).version);
    assert_eq!(Version::new(1, 0, 1), versions.remove(0).version);

    let request = VersionReq::parse("<1.0.1")?;
    let mut versions = PackageModel::find_versions(
        &pool,
        &mock_namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    assert!(versions.len() == 1);
    assert_eq!(Version::new(1, 0, 0), versions.remove(0).version);

    let request = VersionReq::parse(">1.0.0")?;
    let mut versions = PackageModel::find_versions(
        &pool,
        &mock_namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    assert!(versions.len() == 1);
    assert_eq!(Version::new(1, 0, 1), versions.remove(0).version);

    let request = VersionReq::parse("<=1.0.1")?;
    let mut versions = PackageModel::find_versions(
        &pool,
        &mock_namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    assert!(versions.len() == 2);
    assert_eq!(Version::new(1, 0, 0), versions.remove(0).version);
    assert_eq!(Version::new(1, 0, 1), versions.remove(0).version);

    let request = VersionReq::parse(">=1.0.0")?;
    let mut versions = PackageModel::find_versions(
        &pool,
        &mock_namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    assert!(versions.len() == 2);
    assert_eq!(Version::new(1, 0, 0), versions.remove(0).version);
    assert_eq!(Version::new(1, 0, 1), versions.remove(0).version);

    Ok(())
}
