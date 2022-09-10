use k256::ecdsa::SigningKey;
use secrecy::ExposeSecret;
use std::path::PathBuf;

use web3_keystore::{decrypt, KeyStore};

use crate::{input::read_password, Error, Result};

/// Read a keystore file into a signing key.
pub(crate) fn read_keystore_file(key: PathBuf) -> Result<SigningKey> {
    if !key.is_file() {
        return Err(Error::NotFile(key));
    }

    let buffer = std::fs::read(key)?;
    let keystore: KeyStore = serde_json::from_slice(&buffer)?;

    let password = read_password(Some("Keystore passphrase: "))?;

    let key = decrypt(&keystore, password.expose_secret())?;
    let signing_key = SigningKey::from_bytes(&key)?;
    Ok(signing_key)
}
