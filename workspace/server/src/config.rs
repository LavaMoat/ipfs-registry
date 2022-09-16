use indexmap::set::IndexSet;
use serde::Deserialize;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use url::Url;
use web3_address::ethereum::Address;
use k256::ecdsa::SigningKey;
use web3_keystore::{decrypt, KeyStore};

use crate::{Error, Result};
use ipfs_registry_core::RegistryKind;

const KEYSTORE_PASSWORD_ENV: &str = "IPKG_WEBHOOK_KEYSTORE_PASSWORD";

#[derive(Deserialize)]
pub struct ServerConfig {
    /// Configuration for the database.
    #[serde(default)]
    pub database: DatabaseConfig,

    /// Configuration for the primary storage layer.
    #[serde(default)]
    pub storage: StorageConfig,

    /// Package registry configuration.
    #[serde(default)]
    pub registry: RegistryConfig,

    /// Configuration for webhooks.
    pub webhooks: Option<WebHookConfig>,

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
            database: Default::default(),
            registry: Default::default(),
            webhooks: Default::default(),
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

        if let Some(hooks) = config.webhooks.as_mut() {
            if hooks.key.is_relative() {
                hooks.key = dir.join(&hooks.key);
            }
            hooks.key = hooks.key.canonicalize()?;

            let buffer = std::fs::read(&hooks.key)?;
            let keystore: KeyStore = serde_json::from_slice(&buffer)?;

            let password = std::env::var(KEYSTORE_PASSWORD_ENV).ok()
                .ok_or(Error::WebHookKeystorePassword)?;

            let key = decrypt(&keystore, &password)?;
            let signing_key = SigningKey::from_bytes(&key)?;
            hooks.signing_key = Some(signing_key);
        }

        let mut layers = IndexSet::new();
        for mut layer in config.storage.layers.drain(..) {
            if let LayerConfig::File { directory } = &mut layer {
                // Make relative where necessary
                if directory.is_relative() {
                    *directory = dir.join(directory.clone());
                }

                // Resolve symlinks now
                *directory = directory.canonicalize()?;

                if !directory.is_dir() {
                    return Err(Error::NotDirectory(directory.clone()));
                }
            }
            layers.insert(layer);
        }

        config.storage.layers = layers;

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

impl From<LayerConfig> for StorageConfig {
    fn from(layer: LayerConfig) -> Self {
        let mut layers = IndexSet::new();
        layers.insert(layer);
        Self { layers }
    }
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    /// URL for database connections.
    pub url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite::memory:".to_owned(),
            //url: "sqlite:ipfs_registry.db".to_owned(),
        }
    }
}

fn default_body_limit() -> usize {
    1024 * 1024 * 16
}

fn default_mime() -> String {
    String::from("application/gzip")
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct RegistryConfig {
    /// Maximum size of body requests.
    #[serde(default = "default_body_limit")]
    pub body_limit: usize,
    /// Expected mime type for packages.
    #[serde(default = "default_mime")]
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
            body_limit: default_body_limit(),
            mime: default_mime(),
            kind: Default::default(),
            allow: None,
            deny: None,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct WebHookConfig {
    /// Path to the signing key for webhooks.
    pub key: PathBuf,
    /// Endpoints to call for each webhook event.
    pub endpoints: Vec<Url>,
    /// Signing key decrypted from the keystore.
    #[serde(skip)]
    pub(crate) signing_key: Option<SigningKey>,
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
        /// Prefix for objects.
        #[serde(default)]
        prefix: String,
    },
    Memory {
        memory: bool,
    },
    File {
        directory: PathBuf,
    },
}

impl Default for LayerConfig {
    fn default() -> Self {
        Self::Ipfs {
            url: Url::parse("http://localhost:5001").unwrap(),
        }
    }
}
