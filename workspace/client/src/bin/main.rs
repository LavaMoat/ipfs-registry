use clap::{Parser, Subcommand};
use mime::Mime;
use semver::Version;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;
use web3_address::ethereum::Address;

use ipfs_registry_client::Result;

/// Client for the IPFS package registry server.
#[derive(Parser, Debug)]
#[clap(name = "ipkg-client", author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Generate a signing key.
    Keygen {
        /// Write the keystore to file.
        #[clap(parse(from_os_str))]
        file: PathBuf,
    },
    /// Publish a package.
    Publish {
        /// Server URL.
        #[clap(short, long, default_value = "http://127.0.0.1:9060")]
        server: Url,

        /// Media type for the file.
        #[clap(short, long, default_value = "application/gzip")]
        mime: Mime,

        /// Keystore for the signing key.
        #[clap(short, long, parse(from_os_str))]
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

        /// Address of the package owner.
        #[clap(short, long)]
        address: Address,

        /// Name of the package.
        #[clap(short, long)]
        name: String,

        /// Package version.
        #[clap(short, long)]
        version: Version,

        /// Write package to file.
        #[clap(parse(from_os_str))]
        file: PathBuf,
    },
}

async fn run() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Command::Keygen { file } => {
            ipfs_registry_client::keygen(file).await?;
        }
        Command::Publish {
            server,
            mime,
            key,
            file,
        } => {
            ipfs_registry_client::publish(server, mime, key, file).await?;
        }
        Command::Fetch {
            server,
            address,
            name,
            version,
            file,
        } => {
            ipfs_registry_client::fetch(server, address, name, version, file)
                .await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
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
