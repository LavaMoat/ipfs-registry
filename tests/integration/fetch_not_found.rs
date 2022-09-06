use anyhow::Result;
use serial_test::serial;

use semver::Version;

use ipfs_registry_client::fetch;
use tempfile::NamedTempFile;

use crate::test_utils::*;

#[tokio::test]
#[serial]
async fn integration_fetch_not_found() -> Result<()> {
    // Spawn the server
    let (rx, _handle) = spawn(default_server_config())?;
    let _ = rx.await?;

    let server_url = server();

    let tmp = NamedTempFile::new()?;
    let output = tmp.path().to_path_buf();

    // Fetch expects the file not to exist
    std::fs::remove_file(&output)?;

    let result = fetch(
        server_url,
        "0x0000000000000000000000000000000000000000".to_owned(),
        "foo-name".to_owned(),
        Version::new(1, 0, 0),
        output.clone(),
    )
    .await;

    assert!(result.is_err());

    let is_not_found = if let Err(
        ipfs_registry_client::Error::ResponseCode(code),
    ) = result
    {
        code == 404
    } else {
        false
    };

    assert!(is_not_found);

    Ok(())
}
