use anyhow::Result;
use serial_test::serial;
use std::path::PathBuf;

use crate::test_utils::*;
use semver::Version;

use ipfs_registry_client::RegistryClient;
use ipfs_registry_core::{Namespace, PackageName};

use k256::ecdsa::SigningKey;

#[tokio::test]
#[serial]
async fn integration_publish_ok() -> Result<()> {
    // Spawn the server
    let (rx, _handle) = spawn(default_server_config())?;
    let _ = rx.await?;

    let server_url = server();

    let file = PathBuf::from("fixtures/mock-package-1.0.0.tgz");
    let mime: mime::Mime = "application/gzip".parse()?;
    let signing_key = SigningKey::random(&mut rand::thread_rng());

    let namespace = Namespace::new_unchecked("mock-namespace");

    prepare_mock_namespace(&server_url, &signing_key, &namespace).await?;

    let receipt = RegistryClient::publish_file(
        server_url,
        namespace,
        mime,
        signing_key,
        file,
    )
    .await?;

    assert_eq!(
        PackageName::new_unchecked("mock-package"),
        receipt.artifact.package.name
    );
    assert_eq!(Version::new(1, 0, 0), receipt.artifact.package.version);

    Ok(())
}
