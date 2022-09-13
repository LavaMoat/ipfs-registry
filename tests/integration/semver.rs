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

    let mock_package = PackageName::new_unchecked("mock-package");

    // Verify for publishing
    let (publisher_record, namespace_record) =
        PackageModel::can_write_namespace(&pool, &address, &namespace)
            .await?;

    // Pre 1.0.0 releases
    let dev_release_1 = mock_pointer(Some(Version::new(0, 1, 0)))?;
    let dev_release_2 = mock_pointer(Some(Version::new(0, 1, 1)))?;
    let dev_release_3 = mock_pointer(Some(Version::new(0, 2, 0)))?;
    // 1.0.0
    let first_release = mock_pointer(None)?;
    // 1.0.1
    let patch_release = mock_pointer(Some(Version::new(1, 0, 1)))?;
    // 1.1.0
    let point_release = mock_pointer(Some(Version::new(1, 1, 0)))?;
    // 2.0.0-alpha.1
    let next_pre_release_1 =
        mock_pointer(Some(Version::parse("2.0.0-alpha.1")?))?;
    // 2.0.0-alpha.2
    let next_pre_release_2 =
        mock_pointer(Some(Version::parse("2.0.0-alpha.2")?))?;

    // PUBLISH

    let result = PackageModel::insert(
        &pool,
        &publisher_record,
        &namespace_record,
        &address,
        &dev_release_1,
    )
    .await?;
    assert!(result > 0);
    let result = PackageModel::insert(
        &pool,
        &publisher_record,
        &namespace_record,
        &address,
        &dev_release_2,
    )
    .await?;
    assert!(result > 0);
    let result = PackageModel::insert(
        &pool,
        &publisher_record,
        &namespace_record,
        &address,
        &dev_release_3,
    )
    .await?;
    assert!(result > 0);

    let result = PackageModel::insert(
        &pool,
        &publisher_record,
        &namespace_record,
        &address,
        &first_release,
    )
    .await?;
    assert!(result > 0);

    let result = PackageModel::insert(
        &pool,
        &publisher_record,
        &namespace_record,
        &address,
        &patch_release,
    )
    .await?;
    assert!(result > 0);

    let result = PackageModel::insert(
        &pool,
        &publisher_record,
        &namespace_record,
        &address,
        &point_release,
    )
    .await?;
    assert!(result > 0);

    let result = PackageModel::insert(
        &pool,
        &publisher_record,
        &namespace_record,
        &address,
        &next_pre_release_1,
    )
    .await?;
    assert!(result > 0);

    let result = PackageModel::insert(
        &pool,
        &publisher_record,
        &namespace_record,
        &address,
        &next_pre_release_2,
    )
    .await?;
    assert!(result > 0);

    // FIND VERSIONS

    let request = VersionReq::parse("=1.0.0")?;
    let versions = PackageModel::find_versions(
        &pool,
        &namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    let mut versions = versions.records;
    assert!(versions.len() == 1);
    assert_eq!(Version::new(1, 0, 0), versions.remove(0).version);

    let request = VersionReq::parse("=1.0.0, =1.0.1")?;
    let versions = PackageModel::find_versions(
        &pool,
        &namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    let mut versions = versions.records;
    assert!(versions.len() == 2);
    assert_eq!(Version::new(1, 0, 0), versions.remove(0).version);
    assert_eq!(Version::new(1, 0, 1), versions.remove(0).version);

    let request = VersionReq::parse("^0")?;
    let versions = PackageModel::find_versions(
        &pool,
        &namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    let mut versions = versions.records;
    assert!(versions.len() == 3);
    assert_eq!(Version::new(0, 1, 0), versions.remove(0).version);
    assert_eq!(Version::new(0, 1, 1), versions.remove(0).version);
    assert_eq!(Version::new(0, 2, 0), versions.remove(0).version);

    let request = VersionReq::parse("^0.1")?;
    let versions = PackageModel::find_versions(
        &pool,
        &namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    let mut versions = versions.records;
    println!("{}", versions.len());
    assert!(versions.len() == 2);
    assert_eq!(Version::new(0, 1, 0), versions.remove(0).version);
    assert_eq!(Version::new(0, 1, 1), versions.remove(0).version);

    let request = VersionReq::parse("~1.0.0")?;
    let versions = PackageModel::find_versions(
        &pool,
        &namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    let mut versions = versions.records;
    assert!(versions.len() == 2);
    assert_eq!(Version::new(1, 0, 0), versions.remove(0).version);
    assert_eq!(Version::new(1, 0, 1), versions.remove(0).version);

    let request = VersionReq::parse("1.0.*")?;
    let versions = PackageModel::find_versions(
        &pool,
        &namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    let mut versions = versions.records;
    assert!(versions.len() == 2);
    assert_eq!(Version::new(1, 0, 0), versions.remove(0).version);
    assert_eq!(Version::new(1, 0, 1), versions.remove(0).version);

    let request = VersionReq::parse("1.*.*")?;
    let versions = PackageModel::find_versions(
        &pool,
        &namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    let mut versions = versions.records;
    assert!(versions.len() == 3);
    assert_eq!(Version::new(1, 0, 0), versions.remove(0).version);
    assert_eq!(Version::new(1, 0, 1), versions.remove(0).version);
    assert_eq!(Version::new(1, 1, 0), versions.remove(0).version);

    let request = VersionReq::parse("=1")?;
    let versions = PackageModel::find_versions(
        &pool,
        &namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    let mut versions = versions.records;
    assert!(versions.len() == 3);
    assert_eq!(Version::new(1, 0, 0), versions.remove(0).version);
    assert_eq!(Version::new(1, 0, 1), versions.remove(0).version);
    assert_eq!(Version::new(1, 1, 0), versions.remove(0).version);

    let request = VersionReq::parse("<1.0.1")?;
    let versions = PackageModel::find_versions(
        &pool,
        &namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    let mut versions = versions.records;
    assert!(versions.len() == 4);
    assert_eq!(Version::new(0, 1, 0), versions.remove(0).version);
    assert_eq!(Version::new(0, 1, 1), versions.remove(0).version);
    assert_eq!(Version::new(0, 2, 0), versions.remove(0).version);
    assert_eq!(Version::new(1, 0, 0), versions.remove(0).version);

    let request = VersionReq::parse(">1.0.0")?;
    let versions = PackageModel::find_versions(
        &pool,
        &namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    let mut versions = versions.records;
    assert!(versions.len() == 4);
    assert_eq!(Version::new(1, 0, 1), versions.remove(0).version);
    assert_eq!(Version::new(1, 1, 0), versions.remove(0).version);
    assert_eq!(Version::parse("2.0.0-alpha.1")?, versions.remove(0).version);
    assert_eq!(Version::parse("2.0.0-alpha.2")?, versions.remove(0).version);

    let request = VersionReq::parse("<=1.0.1")?;
    let versions = PackageModel::find_versions(
        &pool,
        &namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    let mut versions = versions.records;
    assert!(versions.len() == 5);
    assert_eq!(Version::new(0, 1, 0), versions.remove(0).version);
    assert_eq!(Version::new(0, 1, 1), versions.remove(0).version);
    assert_eq!(Version::new(0, 2, 0), versions.remove(0).version);
    assert_eq!(Version::new(1, 0, 0), versions.remove(0).version);
    assert_eq!(Version::new(1, 0, 1), versions.remove(0).version);

    let request = VersionReq::parse(">=1.0.0")?;
    let versions = PackageModel::find_versions(
        &pool,
        &namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    let mut versions = versions.records;
    assert!(versions.len() == 5);
    assert_eq!(Version::new(1, 0, 0), versions.remove(0).version);
    assert_eq!(Version::new(1, 0, 1), versions.remove(0).version);
    assert_eq!(Version::new(1, 1, 0), versions.remove(0).version);
    assert_eq!(Version::parse("2.0.0-alpha.1")?, versions.remove(0).version);
    assert_eq!(Version::parse("2.0.0-alpha.2")?, versions.remove(0).version);

    let request = VersionReq::parse(">=2")?;
    let versions = PackageModel::find_versions(
        &pool,
        &namespace,
        &mock_package,
        &request,
        &Default::default(),
    )
    .await?;
    let mut versions = versions.records;
    assert!(versions.len() == 2);
    assert_eq!(Version::parse("2.0.0-alpha.1")?, versions.remove(0).version);
    assert_eq!(Version::parse("2.0.0-alpha.2")?, versions.remove(0).version);

    Ok(())
}
