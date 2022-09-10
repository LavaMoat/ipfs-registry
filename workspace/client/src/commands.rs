use k256::ecdsa::SigningKey;
use mime::Mime;

use secrecy::ExposeSecret;
use std::path::PathBuf;
use url::Url;
use web3_address::ethereum::Address;
use web3_keystore::encrypt;

use ipfs_registry_core::{PackageKey, Receipt};

use crate::{helpers, input, Error, RegistryClient, Result};

/// Publish a package.
pub async fn publish(
    server: Url,
    mime: Mime,
    key: PathBuf,
    file: PathBuf,
) -> Result<Receipt> {
    let signing_key = helpers::read_keystore_file(key)?;
    RegistryClient::publish_file(server, mime, signing_key, file).await
}

/// Signup for publishing.
pub async fn signup(server: Url, key: PathBuf) -> Result<()> {
    let signing_key = helpers::read_keystore_file(key)?;
    RegistryClient::signup(server, signing_key).await
}

/// Register a namespace.
pub async fn register(
    server: Url,
    key: PathBuf,
    namespace: String,
) -> Result<()> {
    let signing_key = helpers::read_keystore_file(key)?;
    RegistryClient::register(server, signing_key, namespace).await
}

/// Download a package and write it to file.
pub async fn fetch(
    server: Url,
    key: PackageKey,
    file: PathBuf,
) -> Result<PathBuf> {
    RegistryClient::fetch_file(server, key, file).await
}

/// Generate a signing key and write the result to file.
pub async fn keygen(dir: PathBuf) -> Result<Address> {
    if !dir.is_dir() {
        return Err(Error::NotDirectory(dir));
    }

    let password = input::read_password(None)?;
    let confirm = input::read_password(Some("Confirm password: "))?;

    if password.expose_secret() != confirm.expose_secret() {
        return Err(Error::PasswordMismatch);
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
