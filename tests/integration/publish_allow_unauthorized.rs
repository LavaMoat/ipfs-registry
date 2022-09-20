use anyhow::Result;
use serial_test::serial;
use std::{collections::HashSet, path::PathBuf};

use crate::test_utils::*;

use ipfs_registry_client::RegistryClient;
use ipfs_registry_core::Namespace;
use ipfs_registry_server::config::RegistryConfig;

use web3_address::ethereum::Address;

use k256::ecdsa::SigningKey;

#[tokio::test]
#[serial]
async fn integration_publish_allow_unauthorized() -> Result<()> {
    // Allowed address
    let allowed_key = SigningKey::random(&mut rand::thread_rng());
    let verifying_key = allowed_key.verifying_key();
    let address: Address = verifying_key.into();

    // Create a new signing key that does not exist in
    // the allowed address
    let file = PathBuf::from("fixtures/mock-package-1.0.0.tgz");
    let mime: mime::Mime = "application/gzip".parse()?;
    let signing_key = SigningKey::random(&mut rand::thread_rng());

    let mut registry_config: RegistryConfig = Default::default();
    let mut allow = HashSet::new();
    allow.insert(address);
    registry_config.allow = Some(allow);

    // Spawn the server
    let (rx, _handle) = spawn(registry_server_config(registry_config))?;
    let _ = rx.await?;

    let server_url = server();

    let namespace = Namespace::new_unchecked("mock-namespace");

    prepare_mock_namespace(&server_url, &signing_key, &namespace).await?;

    let result = RegistryClient::publish_file(
        server_url,
        signing_key,
        namespace,
        mime,
        file,
    )
    .await;

    assert!(result.is_err());

    let is_unauthorized = if let Err(
        ipfs_registry_client::Error::ResponseCode(code),
    ) = result
    {
        code == 401
    } else {
        false
    };

    assert!(is_unauthorized);

    Ok(())
}
