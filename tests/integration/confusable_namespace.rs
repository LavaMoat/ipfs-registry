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
async fn integration_confusable_namespace() -> Result<()> {
    // Spawn the server
    let (rx, _handle) = spawn(default_server_config())?;
    let _ = rx.await?;

    let server_url = server();

    let file = PathBuf::from("fixtures/mock-package-1.0.0.tgz");
    let mime: mime::Mime = "application/gzip".parse()?;
    let signing_key = SigningKey::random(&mut rand::thread_rng());

    let namespace = Namespace::new_unchecked("mock-namespace");

    // 03BF GREEK SMALL LETTER OMICRO at index 1
    let confusable = Namespace::new_unchecked("mοck-namespace");

    prepare_mock_namespace(&server_url, &signing_key, &namespace).await?;

    let result =
        prepare_mock_namespace(&server_url, &signing_key, &confusable).await;

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
