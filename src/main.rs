use std::path::PathBuf;

use clap::{Parser, Subcommand};
use mime::Mime;
use serde_json::json;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;
use web3_address::ethereum::Address;

use ipfs_registry::Result;
use ipfs_registry_core::{
    AnyRef, Namespace, PackageKey, PackageName, PathRef,
};
use ipfs_registry_database::{Pager, SortOrder, default_limit};

/// Print an ok response to stdout.
fn ok_response() -> Result<()> {
    serde_json::to_writer_pretty(std::io::stdout(), &json!({"ok": true}))?;
    Ok(())
}

/// Signed package registry server.
#[derive(Parser, Debug)]
#[clap(name = "ipkg", author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Generate a signing key.
    Keygen {
        /// Write the keystore file to directory.
        #[clap(parse(from_os_str))]
        dir: PathBuf,
    },
    /// Signup the public key for publishing.
    Signup {
        /// Server URL.
        #[clap(short, long, default_value = "http://127.0.0.1:9060")]
        server: Url,

        /// Keystore for the signing key.
        #[clap(
            short,
            long,
            parse(from_os_str),
            env = "IPKG_KEYSTORE",
            hide_env = true
        )]
        key: PathBuf,
    },
    /// Register a namespace.
    Register {
        /// Server URL.
        #[clap(short, long, default_value = "http://127.0.0.1:9060")]
        server: Url,

        /// Keystore for the signing key.
        #[clap(
            short,
            long,
            parse(from_os_str),
            env = "IPKG_KEYSTORE",
            hide_env = true
        )]
        key: PathBuf,

        /// Namespace to register.
        namespace: Namespace,
    },
    /// Publish a package.
    Publish {
        /// Server URL.
        #[clap(short, long, default_value = "http://127.0.0.1:9060")]
        server: Url,

        /// Namespace for packages.
        #[clap(short, long)]
        namespace: Namespace,

        /// Media type for the file.
        #[clap(short, long, default_value = "application/gzip")]
        mime: Mime,

        /// Keystore for the signing key.
        #[clap(
            short,
            long,
            parse(from_os_str),
            env = "IPKG_KEYSTORE",
            hide_env = true
        )]
        key: PathBuf,

        /// File to publish.
        #[clap(parse(from_os_str))]
        file: PathBuf,
    },
    /// Download a package.
    Fetch {
        /// Server URL.
        #[clap(short, long, default_value = "http://127.0.0.1:9060")]
        server: Url,

        /// Package identifier.
        id: PackageKey,

        /// Write package to file.
        #[clap(parse(from_os_str))]
        file: PathBuf,
    },
    /// Manage namespace users.
    User {
        #[clap(subcommand)]
        cmd: User,
    },
    /// Yank a package version.
    Yank {
        /// Server URL.
        #[clap(short, long, default_value = "http://127.0.0.1:9060")]
        server: Url,

        /// Keystore for the signing key.
        #[clap(
            short,
            long,
            parse(from_os_str),
            env = "IPKG_KEYSTORE",
            hide_env = true
        )]
        key: PathBuf,

        /// Package identifier.
        id: PackageKey,

        /// Reason for yanking the version.
        message: Option<String>,
    },
    /// Deprecate a package.
    Deprecate {
        /// Server URL.
        #[clap(short, long, default_value = "http://127.0.0.1:9060")]
        server: Url,

        /// Keystore for the signing key.
        #[clap(
            short,
            long,
            parse(from_os_str),
            env = "IPKG_KEYSTORE",
            hide_env = true
        )]
        key: PathBuf,

        /// Package path.
        path: PathRef,

        /// Reason for yanking the version.
        message: Option<String>,
    },
    /// Get information about a specific package version.
    Get {
        /// Server URL.
        #[clap(short, long, default_value = "http://127.0.0.1:9060")]
        server: Url,

        /// Identifier for a namespace, package or version.
        target: AnyRef,
    },
    /// List packages and versions.
    #[clap(alias = "ls")]
    List {
        /// Server URL.
        #[clap(short, long, default_value = "http://127.0.0.1:9060")]
        server: Url,

        #[clap(short, long)]
        offset: Option<i64>,

        #[clap(short, long)]
        limit: Option<i64>,

        #[clap(short = 'S', long)]
        sort: Option<SortOrder>,

        /// Path to a namespace or package.
        path: PathRef,
    },
    /// Start a server.
    Server {
        /// Bind to host:port.
        #[clap(short, long, default_value = "127.0.0.1:9060")]
        bind: String,

        /// Config file to load.
        #[clap(short, long, parse(from_os_str))]
        config: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum User {
    /// Add user access to a namespace.
    Add {
        /// Server URL.
        #[clap(short, long, default_value = "http://127.0.0.1:9060")]
        server: Url,

        /// Make the user an administrator.
        #[clap(short, long)]
        admin: bool,

        /// Restrict the user to target package.
        #[clap(short, long)]
        package: Option<PackageName>,

        /// Keystore for the signing key.
        #[clap(
            short,
            long,
            parse(from_os_str),
            env = "IPKG_KEYSTORE",
            hide_env = true
        )]
        key: PathBuf,

        /// Target namespace.
        #[clap(short, long)]
        namespace: Namespace,

        /// Address of the user.
        user: Address,
    },

    /// Remove user access from a namespace.
    Remove {
        /// Server URL.
        #[clap(short, long, default_value = "http://127.0.0.1:9060")]
        server: Url,

        /// Keystore for the signing key.
        #[clap(
            short,
            long,
            parse(from_os_str),
            env = "IPKG_KEYSTORE",
            hide_env = true
        )]
        key: PathBuf,

        /// Target namespace.
        #[clap(short, long)]
        namespace: Namespace,

        /// Address of the user.
        user: Address,
    },

    /// Grant user access to a package.
    Grant {
        /// Server URL.
        #[clap(short, long, default_value = "http://127.0.0.1:9060")]
        server: Url,

        /// Keystore for the signing key.
        #[clap(
            short,
            long,
            parse(from_os_str),
            env = "IPKG_KEYSTORE",
            hide_env = true
        )]
        key: PathBuf,

        /// Target namespace.
        #[clap(short, long)]
        namespace: Namespace,

        /// Address of the user.
        user: Address,

        /// Grant access to target package.
        package: PackageName,
    },

    /// Revoke user access to a package.
    Revoke {
        /// Server URL.
        #[clap(short, long, default_value = "http://127.0.0.1:9060")]
        server: Url,

        /// Keystore for the signing key.
        #[clap(
            short,
            long,
            parse(from_os_str),
            env = "IPKG_KEYSTORE",
            hide_env = true
        )]
        key: PathBuf,

        /// Target namespace.
        #[clap(short, long)]
        namespace: Namespace,

        /// Address of the user.
        user: Address,

        /// Revoke access to target package.
        package: PackageName,
    },
}

async fn run() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Command::Keygen { dir } => {
            let address = ipfs_registry_client::keygen(dir).await?;
            serde_json::to_writer_pretty(std::io::stdout(), &address)?;
        }
        Command::Signup { server, key } => {
            let doc = ipfs_registry_client::signup(server, key).await?;
            serde_json::to_writer_pretty(std::io::stdout(), &doc)?;
        }
        Command::Register {
            server,
            key,
            namespace,
        } => {
            let doc = ipfs_registry_client::register(server, key, namespace)
                .await?;
            serde_json::to_writer_pretty(std::io::stdout(), &doc)?;
        }
        Command::Publish {
            server,
            namespace,
            mime,
            key,
            file,
        } => {
            let doc = ipfs_registry_client::publish(
                server, namespace, mime, key, file,
            )
            .await?;
            serde_json::to_writer_pretty(std::io::stdout(), &doc)?;
        }
        Command::Fetch { server, id, file } => {
            let file = ipfs_registry_client::fetch(server, id, file).await?;
            let size = file.metadata()?.len();
            tracing::info!(file = ?file, size = ?size);
        }
        Command::User { cmd } => match cmd {
            User::Add {
                server,
                key,
                namespace,
                user,
                admin,
                package,
            } => {
                ipfs_registry_client::add_user(
                    server, key, namespace, user, admin, package,
                )
                .await?;
                ok_response()?;
            }
            User::Remove {
                server,
                key,
                namespace,
                user,
            } => {
                ipfs_registry_client::remove_user(
                    server, key, namespace, user,
                )
                .await?;
                ok_response()?;
            }
            User::Grant {
                server,
                key,
                namespace,
                package,
                user,
            } => {
                ipfs_registry_client::access_control(
                    server, key, namespace, package, user, true,
                )
                .await?;
                ok_response()?;
            }
            User::Revoke {
                server,
                key,
                namespace,
                package,
                user,
            } => {
                ipfs_registry_client::access_control(
                    server, key, namespace, package, user, false,
                )
                .await?;
                ok_response()?;
            }
        },
        Command::Yank {
            server,
            key,
            id,
            message,
        } => {
            let message = message.unwrap_or(String::new());
            ipfs_registry_client::yank(server, key, id, message).await?;
            ok_response()?;
        }
        Command::Deprecate {
            server,
            key,
            path,
            message,
        } => {
            let (namespace, package): (Namespace, PackageName) =
                path.try_into()?;
            let message = message.unwrap_or(String::new());
            ipfs_registry_client::deprecate(
                server, key, namespace, package, message,
            )
            .await?;
            ok_response()?;
        }
        Command::Get { server, target } => {
            let doc = ipfs_registry_client::get(server, target).await?;
            serde_json::to_writer_pretty(std::io::stdout(), &doc)?;
        }
        Command::List {
            server,
            path,
            offset,
            limit,
            sort,
        } => {
            let pager = Pager {
                offset: offset.unwrap_or_default(),
                limit: limit.unwrap_or_else(default_limit),
                sort: sort.unwrap_or_default(),
            };
            let doc = ipfs_registry_client::list(server, path, pager).await?;
            serde_json::to_writer_pretty(std::io::stdout(), &doc)?;
        }
        Command::Server { bind, config } => {
            ipfs_registry_server::start(bind, config).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,sqlx::query=warn".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    match run().await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("{}", e);
        }
    }
    Ok(())
}
