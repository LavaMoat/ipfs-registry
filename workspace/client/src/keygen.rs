use k256::ecdsa::SigningKey;
use std::path::PathBuf;
use web3_keystore::encrypt;
use web3_address::ethereum::Address;

use secrecy::ExposeSecret;

use crate::{Error, Result, input::read_password};

/// Generate a signing key and write the result to file.
pub async fn keygen(dir: PathBuf) -> Result<Address> {
    if !dir.is_dir() {
        return Err(Error::NotDirectory(dir))
    }

    let password = read_password(None)?;
    let confirm = read_password(Some("Confirm password: "))?;

    if password.expose_secret() != confirm.expose_secret() {
        return Err(Error::PasswordMismatch)
    }

    let key = SigningKey::random(&mut rand::thread_rng());
    let public_key = key.verifying_key();
    let address: Address = public_key.into();

    let keystore = encrypt(
        &mut rand::thread_rng(),
        key.to_bytes(),
        password.expose_secret(),
        Some(address.to_string()),
    )?;

    let buffer = serde_json::to_vec_pretty(&keystore)?;
    let file = dir.join(format!("{}.json", address));
    std::fs::write(file, buffer)?;

    Ok(address)
}
