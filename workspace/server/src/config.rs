use indexmap::set::IndexSet;
use serde::Deserialize;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use url::Url;
use web3_address::ethereum::Address;

use crate::{Error, Result};
use ipfs_registry_core::RegistryKind;

#[derive(Deserialize)]
pub struct ServerConfig {
    /// Configuration for the primary storage layer.
    #[serde(default)]
    pub storage: StorageConfig,

    /// Package registry configuration.
    #[serde(default)]
    pub registry: RegistryConfig,

    /// Configuration for TLS encryption.
    pub tls: Option<TlsConfig>,

    /// Configuration for CORS.
    pub cors: Option<CorsConfig>,

    /// Path the file was loaded from used to determine
    /// relative paths.
    #[serde(skip)]
    file: Option<PathBuf>,
}

impl ServerConfig {

    /// Create a new server config.
    pub fn new(storage: StorageConfig) -> Self {
        Self {
            storage,
            registry: Default::default(),
            tls: None,
            cors: None,
            file: None,
        }
    }

    /// Load a configuration file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().exists() {
            return Err(Error::NotFile(path.as_ref().to_path_buf()));
        }

        let contents = std::fs::read_to_string(path.as_ref())?;
        let mut config: ServerConfig = toml::from_str(&contents)?;
        config.file = Some(path.as_ref().canonicalize()?);

        if config.storage.layers.is_empty() {
            return Err(Error::NoStorageLayers);
        }

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

/// Storage configuration.
#[derive(Debug, Deserialize)]
pub struct StorageConfig {
    /// Collection of storage layers.
    pub layers: IndexSet<LayerConfig>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        let mut layers = IndexSet::new();
        layers.insert(Default::default());
        Self { layers }
    }
}

#[derive(Debug, Deserialize)]
pub struct RegistryConfig {
    /// Maximum size of body requests.
    pub body_limit: usize,
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
            body_limit: 1024 * 1024 * 16,
            mime: String::from("application/gzip"),
            kind: RegistryKind::Npm,
            allow: None,
            deny: None,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct TlsConfig {
    /// Path to the certificate.
    pub cert: PathBuf,
    /// Path to the certificate key file.
    pub key: PathBuf,
}

#[derive(Debug, Default, Deserialize)]
pub struct CorsConfig {
    /// List of additional CORS origins for the server.
    pub origins: Vec<Url>,
}

#[derive(Debug, Clone, Deserialize, Hash, Eq, PartialEq)]
#[serde(untagged)]
pub enum LayerConfig {
    Ipfs {
        /// URL for the IPFS node.
        url: Url,
    },
    Aws {
        /// Profile for authentication.
        profile: String,
        // Region of the bucket.
        region: String,
        /// Bucket name.
        bucket: String,
    },
    Memory {
        memory: bool,
    }
}

impl Default for LayerConfig {
    fn default() -> Self {
        Self::Ipfs {
            url: Url::parse("http://localhost:5001").unwrap(),
        }
    }
}
