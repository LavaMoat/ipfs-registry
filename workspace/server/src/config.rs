use std::{path::{Path, PathBuf}, collections::HashSet};
use serde::{Deserialize, Serialize};
use url::Url;
use web3_address::ethereum::Address;

use crate::{Error, Result};
use ipfs_registry_core::RegistryKind;

#[derive(Serialize, Deserialize)]
pub struct ServerConfig {
    /// Configuration for IPFS.
    #[serde(default)]
    pub ipfs: IpfsConfig,

    /// Package registry configuration.
    #[serde(default)]
    pub registry: RegistryConfig,

    /// Configuration for TLS encryption.
    pub tls: Option<TlsConfig>,

    /// Configuration for the API.
    pub api: ApiConfig,

    /// Path the file was loaded from used to determine
    /// relative paths.
    #[serde(skip)]
    file: Option<PathBuf>,
}

impl ServerConfig {
    /// Load a configuration file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().exists() {
            return Err(Error::NotFile(path.as_ref().to_path_buf()));
        }

        let contents = std::fs::read_to_string(path.as_ref())?;
        let mut config: ServerConfig = toml::from_str(&contents)?;
        config.file = Some(path.as_ref().canonicalize()?);

        let dir = config.directory();

        if let Some(tls) = config.tls.as_mut() {
            if tls.cert.is_relative() {
                tls.cert = dir.join(&tls.cert);
            }
            if tls.key.is_relative() {
                tls.key = dir.join(&tls.key);
            }

            tls.cert = tls.cert.canonicalize()?;
            tls.key = tls.key.canonicalize()?;
        }

        // Sanity check the MIME type
        let _: mime::Mime = config.registry.mime.parse()?;

        Ok(config)
    }

    /// Parent directory of the configuration file.
    fn directory(&self) -> PathBuf {
        self.file
            .as_ref()
            .unwrap()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpfsConfig {
    /// URL for the IPFS node.
    pub url: Url,
}

impl Default for IpfsConfig {
    fn default() -> Self {
        Self {
            url: Url::parse("http://localhost:5001").unwrap(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Expected mime type for packages.
    pub mime: String,
    /// Indicate the kind of registry.
    pub kind: RegistryKind,
    /// Set of addresses that are allow to publish.
    pub allow: Option<HashSet<Address>>,
    /// Set of addresses that are not allowed to publish.
    pub deny: Option<HashSet<Address>>,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            mime: String::from("application/gzip"),
            kind: RegistryKind::Npm,
            allow: None,
            deny: None,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to the certificate.
    pub cert: PathBuf,
    /// Path to the certificate key file.
    pub key: PathBuf,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ApiConfig {
    /// List of additional CORS origins for the server.
    pub origins: Vec<Url>,
}
