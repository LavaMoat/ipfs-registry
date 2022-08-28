use std::path::PathBuf;
use k256::ecdsa::SigningKey;
use web3_keystore::encrypt;

use web3_address::ethereum::Address;

use crate::Result;

/// Generate a signing key and write the result to file.
pub async fn keygen(file: PathBuf) -> Result<()> {
    let key = SigningKey::random(&mut rand::thread_rng());
    let public_key = key.verifying_key();
    let address: Address = public_key.into();

    // TODO: get password from stdin
    let password = String::from("");

    let keystore = encrypt(
        &mut rand::thread_rng(),
        key.to_bytes(),
        password,
        Some(address.to_string()))?;

    let buffer = serde_json::to_vec_pretty(&keystore)?;
    std::fs::write(file, buffer)?;

    tracing::info!(address = %address);

    Ok(())
}
