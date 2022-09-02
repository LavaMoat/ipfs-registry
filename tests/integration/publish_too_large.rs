use anyhow::Result;
use serial_test::serial;
use std::path::PathBuf;

use crate::test_utils::*;

use ipfs_registry_client::publish::publish_with_key;
use ipfs_registry_server::config::RegistryConfig;

use k256::ecdsa::SigningKey;

#[tokio::test]
#[serial]
async fn integration_publish_too_large() -> Result<()> {
    let mut registry: RegistryConfig = Default::default();
    registry.body_limit = 1024 * 1024 * 2;

    // Spawn the server
    let (rx, _handle) = spawn(registry_server_config(registry))?;
    let _ = rx.await?;

    let server_url = server();

    let file = PathBuf::from("fixtures/payload-too-large-1.0.0.tgz");
    let mime: mime::Mime = "application/gzip".parse()?;
    let signing_key = SigningKey::random(&mut rand::thread_rng());

    let result = publish_with_key(server_url, mime, signing_key, file).await;

    //println!("{:#?}", result);

    assert!(result.is_err());

    let is_too_large = if let Err(
        ipfs_registry_client::Error::ResponseCode(code),
    ) = result
    {
        code == 413
    } else {
        false
    };

    assert!(is_too_large);

    Ok(())
}
