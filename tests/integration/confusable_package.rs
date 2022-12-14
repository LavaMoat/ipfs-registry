use anyhow::Result;
use serial_test::serial;
use std::path::PathBuf;

use crate::test_utils::*;

use ipfs_registry_client::RegistryClient;
use ipfs_registry_core::Namespace;

use k256::ecdsa::SigningKey;

#[tokio::test]
#[serial]
async fn integration_confusable_package() -> Result<()> {
    // Spawn the server
    let (rx, _handle) = spawn(default_server_config())?;
    let _ = rx.await?;

    let server_url = server();

    let file = PathBuf::from("fixtures/mock-package-1.0.0.tgz");

    let mime: mime::Mime = "application/gzip".parse()?;
    let signing_key = SigningKey::random(&mut rand::thread_rng());

    let namespace = Namespace::new_unchecked("mock-namespace");

    prepare_mock_namespace(&server_url, &signing_key, &namespace).await?;

    // Publish the legitimate package
    RegistryClient::publish_file(
        server_url.clone(),
        signing_key.clone(),
        namespace.clone(),
        mime.clone(),
        file,
    )
    .await?;

    // Uses 0430 CYRILLIC SMALL LETTER A for the "a" characters
    //
    // Inside the tarball the actual attack name is "mock-pаckаge" the
    // tarball has a different name so we don't get confused!
    let file = PathBuf::from("fixtures/confusable-pаckаge-1.0.0.tgz");

    // Try to publis the confusable package
    let result = RegistryClient::publish_file(
        server_url,
        signing_key,
        namespace,
        mime,
        file,
    )
    .await;

    let is_conflict = if let Err(ipfs_registry_client::Error::ResponseCode(
        code,
    )) = result
    {
        code == 409
    } else {
        false
    };
    assert!(is_conflict);

    Ok(())
}
