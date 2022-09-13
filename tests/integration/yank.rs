use anyhow::Result;
use serial_test::serial;
use std::path::PathBuf;

use crate::test_utils::*;
use semver::Version;

use ipfs_registry_client::RegistryClient;
use ipfs_registry_core::{Namespace, PackageKey, PackageName};

use k256::ecdsa::SigningKey;

#[tokio::test]
#[serial]
async fn integration_yank() -> Result<()> {
    // Spawn the server
    let (rx, _handle) = spawn(default_server_config())?;
    let _ = rx.await?;

    let server_url = server();

    let file = PathBuf::from("fixtures/mock-package-1.0.0.tgz");
    let mime: mime::Mime = "application/gzip".parse()?;
    let signing_key = SigningKey::random(&mut rand::thread_rng());

    let namespace = Namespace::new_unchecked("mock-namespace");
    let package = PackageName::new_unchecked("mock-package");
    let version = Version::new(1, 0, 0);
    let message = String::from("mock yank message");

    prepare_mock_namespace(&server_url, &signing_key, &namespace).await?;

    let _ = RegistryClient::publish_file(
        server_url.clone(),
        namespace.clone(),
        mime,
        signing_key.clone(),
        file,
    )
    .await?;

    let id = PackageKey::Pointer(
        namespace.clone(),
        package.clone(),
        version.clone(),
    );
    assert!(RegistryClient::yank(
        server_url.clone(),
        signing_key.clone(),
        id.clone(),
        message.clone(),
    )
    .await
    .is_ok());

    let doc = RegistryClient::exact_version(server_url, id).await?;

    assert_eq!(Some(message), doc.yanked);

    Ok(())
}
