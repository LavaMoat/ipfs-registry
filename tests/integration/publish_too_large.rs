use anyhow::Result;
use serial_test::serial;
use std::path::PathBuf;

use crate::test_utils::*;

use ipfs_registry_client::publish::publish_with_key;

use k256::ecdsa::SigningKey;

#[tokio::test]
#[serial]
async fn integration_publish_too_large() -> Result<()> {
    // Spawn the server
    let (rx, _handle) = spawn(default_server_config())?;
    let _ = rx.await?;

    let server_url = server();

    let file = PathBuf::from("fixtures/payload-too-large-1.0.0.tgz");
    let mime: mime::Mime = "application/gzip".parse()?;
    let signing_key = SigningKey::random(&mut rand::thread_rng());

    let result = publish_with_key(server_url, mime, signing_key, file).await;

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
