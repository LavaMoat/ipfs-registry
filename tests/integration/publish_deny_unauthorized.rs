use anyhow::Result;
use serial_test::serial;
use std::{collections::HashSet, path::PathBuf};

use crate::test_utils::*;

use ipfs_registry_client::RegistryClient;
use ipfs_registry_server::config::RegistryConfig;

use web3_address::ethereum::Address;

use k256::ecdsa::SigningKey;

#[tokio::test]
#[serial]
async fn integration_publish_deny_unauthorized() -> Result<()> {
    let file = PathBuf::from("fixtures/mock-package-1.0.0.tgz");
    let mime: mime::Mime = "application/gzip".parse()?;
    let signing_key = SigningKey::random(&mut rand::thread_rng());
    let verifying_key = signing_key.verifying_key();

    let address: Address = verifying_key.into();

    let mut registry_config: RegistryConfig = Default::default();
    let mut deny = HashSet::new();
    deny.insert(address);
    registry_config.deny = Some(deny);

    // Spawn the server
    let (rx, _handle) = spawn(registry_server_config(registry_config))?;
    let _ = rx.await?;

    let server_url = server();

    let result =
        RegistryClient::publish_file(server_url, mime, signing_key, file)
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
