use std::path::PathBuf;
use anyhow::Result;
use serial_test::serial;
use semver::Version;
use k256::ecdsa::SigningKey;

use ipfs_registry_core::PackageKey;
use ipfs_registry_client::{fetch, publish::publish_with_key};
use tempfile::NamedTempFile;

use crate::test_utils::*;

#[tokio::test]
#[serial]
async fn integration_fetch_ok() -> Result<()> {
    // Spawn the server
    let (rx, _handle) = spawn(default_server_config())?;
    let _ = rx.await?;

    let server_url = server();

    let file = PathBuf::from("fixtures/mock-package-1.0.0.tgz");
    let mime: mime::Mime = "application/gzip".parse()?;
    let signing_key = SigningKey::random(&mut rand::thread_rng());

    let receipt =
        publish_with_key(server_url.clone(), mime, signing_key, file).await?;

    assert_eq!("mock-package", receipt.artifact.package.name);
    assert_eq!(Version::new(1, 0, 0), receipt.artifact.package.version);

    let tmp = NamedTempFile::new()?;
    let output = tmp.path().to_path_buf();

    // Fetch expects the file not to exist
    std::fs::remove_file(&output)?;

    let key = PackageKey::Pointer(
        receipt.artifact.namespace.clone(),
        receipt.artifact.package.name.clone(),
        receipt.artifact.package.version.clone(),
    );

    let result = fetch(
        server_url,
        key,
        output.clone(),
    )
    .await?;

    assert_eq!(output, result);

    Ok(())
}
