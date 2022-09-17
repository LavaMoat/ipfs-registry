use k256::ecdsa::SigningKey;
use mime::Mime;
use semver::VersionReq;
use serde::{Deserialize, Serialize};

use secrecy::ExposeSecret;
use std::path::PathBuf;
use url::Url;
use web3_address::ethereum::Address;
use web3_keystore::encrypt;

use ipfs_registry_core::{
    AnyRef, Namespace, PackageKey, PackageName, PathRef, Receipt,
};
use ipfs_registry_database::{
    NamespaceRecord, PackageRecord, Pager, PublisherRecord, ResultSet,
    VersionIncludes, VersionRecord,
};

use crate::{helpers, input, Error, RegistryClient, Result};

/// Enumeration of types for a get operation.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetRecord {
    /// Get a namespace result.
    Namespace(NamespaceRecord),
    /// Get a package result.
    Package(PackageRecord),
    /// Get a version result.
    Version(VersionRecord),
}

/// Enumeration of types for a list operation.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum ListRecord {
    /// Collection of packages.
    Packages(ResultSet<PackageRecord>),
    /// Collection of versions.
    Versions(ResultSet<VersionRecord>),
}

/// Publish a package.
pub async fn publish(
    server: Url,
    namespace: Namespace,
    mime: Mime,
    key: PathBuf,
    file: PathBuf,
) -> Result<Receipt> {
    let signing_key = helpers::read_keystore_file(key)?;
    RegistryClient::publish_file(server, signing_key, namespace, mime, file)
        .await
}

/// Signup for publishing.
pub async fn signup(server: Url, key: PathBuf) -> Result<PublisherRecord> {
    let signing_key = helpers::read_keystore_file(key)?;
    RegistryClient::signup(server, signing_key).await
}

/// Register a namespace.
pub async fn register(
    server: Url,
    key: PathBuf,
    namespace: Namespace,
) -> Result<NamespaceRecord> {
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

/// Yank a package.
pub async fn yank(
    server: Url,
    key: PathBuf,
    id: PackageKey,
    message: String,
) -> Result<()> {
    let signing_key = helpers::read_keystore_file(key)?;
    RegistryClient::yank(server, signing_key, id, message).await
}

/// Deprecate a package.
pub async fn deprecate(
    server: Url,
    key: PathBuf,
    namespace: Namespace,
    package: PackageName,
    message: String,
) -> Result<()> {
    let signing_key = helpers::read_keystore_file(key)?;
    RegistryClient::deprecate(
        server,
        signing_key,
        namespace,
        package,
        message,
    )
    .await
}

/// Get a namespace, package or version.
pub async fn get(
    server: Url,
    target: AnyRef,
    latest: bool,
) -> Result<GetRecord> {
    match target {
        AnyRef::Path(path) => {
            if let Some(package) = path.package() {
                if latest {
                    RegistryClient::latest_version(
                        server,
                        path.namespace().clone(),
                        package.clone(),
                    )
                    .await
                    .map(GetRecord::Version)
                } else {
                    RegistryClient::get_package(
                        server,
                        path.namespace().clone(),
                        package.clone(),
                    )
                    .await
                    .map(GetRecord::Package)
                }
            } else {
                RegistryClient::get_namespace(
                    server,
                    path.namespace().clone(),
                )
                .await
                .map(GetRecord::Namespace)
            }
        }
        AnyRef::Key(id) => RegistryClient::exact_version(server, id)
            .await
            .map(GetRecord::Version),
    }
}

/// List packages or versions.
pub async fn list(
    server: Url,
    path: PathRef,
    pager: Pager,
    include: Option<VersionIncludes>,
    range: Option<VersionReq>,
) -> Result<ListRecord> {
    let namespace = path.namespace().clone();
    let package = path.package().map(|v| v.clone());

    if package.is_some() {
        RegistryClient::list::<ResultSet<VersionRecord>>(
            server, namespace, package, pager, include, range,
        )
        .await
        .map(ListRecord::Versions)
    } else {
        RegistryClient::list::<ResultSet<PackageRecord>>(
            server, namespace, package, pager, include, range,
        )
        .await
        .map(ListRecord::Packages)
    }
}

/// Add a user.
pub async fn add_user(
    server: Url,
    key: PathBuf,
    namespace: Namespace,
    user: Address,
    admin: bool,
    package: Option<PackageName>,
) -> Result<()> {
    let signing_key = helpers::read_keystore_file(key)?;
    RegistryClient::add_user(
        server,
        signing_key,
        namespace,
        user,
        admin,
        package,
    )
    .await
}

/// Remove a user.
pub async fn remove_user(
    server: Url,
    key: PathBuf,
    namespace: Namespace,
    user: Address,
) -> Result<()> {
    let signing_key = helpers::read_keystore_file(key)?;
    RegistryClient::remove_user(server, signing_key, namespace, user).await
}

/// Grant or revoke package access.
pub async fn access_control(
    server: Url,
    key: PathBuf,
    namespace: Namespace,
    package: PackageName,
    user: Address,
    grant: bool,
) -> Result<()> {
    let signing_key = helpers::read_keystore_file(key)?;
    RegistryClient::access_control(
        server,
        signing_key,
        namespace,
        package,
        user,
        grant,
    )
    .await
}
