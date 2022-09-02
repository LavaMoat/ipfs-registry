use std::path::PathBuf;
use anyhow::Result;
use serial_test::serial;

use crate::test_utils::*;
use semver::Version;

use ipfs_registry_client::publish::publish_with_key;

use k256::ecdsa::SigningKey;

#[tokio::test]
#[serial]
async fn integration_package_publish() -> Result<()> {

    // Spawn the server
    let (rx, _handle) = spawn(default_server_config())?;
    let _ = rx.await?;

    let server_url = server();

    let file = PathBuf::from("fixtures/mock-package-1.0.0.tgz");
    let mime: mime::Mime = "application/gzip".parse()?;
    let signing_key = SigningKey::random(&mut rand::thread_rng());

    let receipt = publish_with_key(
        server_url,
        mime,
        signing_key,
        file,
    ).await?;

    assert_eq!("mock-package", receipt.artifact.package.name);
    assert_eq!(Version::new(1, 0, 0), receipt.artifact.package.version);

    Ok(())
}
