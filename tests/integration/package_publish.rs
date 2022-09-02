use anyhow::Result;
use serial_test::serial;

use crate::test_utils::*;

#[tokio::test]
#[serial]
async fn integration_package_publish() -> Result<()> {

    // Spawn the server
    let (rx, _handle) = spawn(default_server_config())?;
    let _ = rx.await?;

    let server_url = server();

    println!("Run integration test {}", server_url);

    Ok(())
}
