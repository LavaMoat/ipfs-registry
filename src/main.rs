use std::path::PathBuf;

use clap::{Parser, Subcommand};
use mime::Mime;
use semver::Version;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;
use web3_address::ethereum::Address;

use ipfs_registry::Result;

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

async fn run() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Command::Keygen { dir } => {
            let address = ipfs_registry_client::keygen(dir).await?;
            tracing::info!(address = %address);
        }
        Command::Publish {
            server,
            mime,
            key,
            file,
        } => {
            let doc = ipfs_registry_client::publish(server, mime, key, file)
                .await?;
            serde_json::to_writer_pretty(std::io::stdout(), &doc)?;
            //tracing::info!(definition = ?definition);
        }
        Command::Fetch {
            server,
            address,
            name,
            version,
            file,
        } => {
            let file = ipfs_registry_client::fetch(
                server, address, name, version, file,
            )
            .await?;
            let size = file.metadata()?.len();
            tracing::info!(file = ?file, size = ?size);
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
