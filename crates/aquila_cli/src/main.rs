use aquila_client::AquilaClient;
use aquila_core::manifest::{AssetInfo, AssetManifest};
use chrono::Utc;
use clap::{Parser, Subcommand};
use rand::Rng;
use rand::distr::Alphanumeric;
use std::collections::HashMap;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "aquila")]
#[command(about = "CLI for Bevy Aquila Asset Server")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Server URL
    #[arg(short, long, default_value = "http://localhost:3000")]
    url: String,

    #[arg(short, long, env = "AQUILA_TOKEN")]
    token: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Upload a single file
    Upload {
        path: PathBuf,
    },
    /// Publish a directory as a new Game Version
    Publish {
        /// The directory containing assets (e.g., "./assets")
        #[arg(long)]
        dir: PathBuf,

        /// The version string (e.g., "0.1.0" or git sha)
        #[arg(long)]
        version: String,
    },
    /// Download a file by hash
    Download {
        hash: String,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Fetch and display a manifest for a specific version
    GetManifest {
        version: String,
    },
    Login,
    GenerateSecret,
    MintToken {
        /// The subject name (e.g. "game_client_v1")
        #[arg(short, long)]
        subject: String,

        /// Duration in seconds (default: 1 year)
        #[arg(long)]
        duration: Option<u64>,

        /// Optional scopes (comma separated, e.g. "read,write")
        #[arg(long, value_delimiter = ',', default_value = "read")]
        scopes: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let client = AquilaClient::new(cli.url.clone(), cli.token.clone());

    match cli.command {
        Commands::GenerateSecret => {
            let secret: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(64)
                .map(char::from)
                .collect();

            println!("🔑 Generated JWT Secret:");
            println!("\n    {}\n", secret);
            println!("Copy this value and set it on your server:");
            println!("set AQUILA_JWT_SECRET=\"{}\"", secret);
        }
        Commands::Login => {
            let login_url = format!("{}/auth/login", cli.url.trim_end_matches('/'));
            println!("🌐 To authenticate, please visit:");
            println!("\n  {}\n", login_url);
            println!("After logging in, copy the 'token' from the JSON response and set it:");
            println!("set AQUILA_TOKEN=\"...\"");
        }
        Commands::Upload { path } => {
            let hash = client.upload_file(&path).await?;
            println!("✅ Upload successful! Hash: {hash}");
        }
        Commands::Publish { dir, version } => {
            println!("🚀 Publishing version '{version}' from {dir:?}...");

            let mut assets = HashMap::new();
            let mut count = 0;

            for entry in WalkDir::new(&dir) {
                let entry = entry?;
                if entry.file_type().is_dir() {
                    continue;
                }

                let path = entry.path();

                let relative_path = path
                    .strip_prefix(&dir)?
                    .to_string_lossy()
                    .replace('\\', "/");

                println!("Processing: {relative_path}");

                let hash = client.upload_file(path).await?;
                let size = entry.metadata()?.len();
                let mime_type = Some(
                    mime_guess::from_path(path)
                        .first_or_octet_stream()
                        .to_string(),
                );

                assets.insert(
                    relative_path,
                    AssetInfo {
                        hash,
                        size,
                        mime_type,
                    },
                );
                count += 1;
            }

            let manifest = AssetManifest {
                version: version.clone(),
                published_at: Utc::now(),
                published_by: whoami::username()?,
                assets,
            };

            client.publish_manifest(&manifest).await?;

            println!("✅ Successfully published version {version} with {count} assets.",);
        }
        Commands::Download { hash, output } => {
            println!("Downloading {hash}...");

            let data = client.download_file(&hash).await?;
            if let Some(parent) = output.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&output, data).await?;

            println!("✅ Saved to {output:?}");
        }
        Commands::GetManifest { version } => {
            println!("🔍 Fetching manifest for version '{}'...", version);
            let manifest = client.fetch_manifest(&version).await?;
            println!("{}", serde_json::to_string_pretty(&manifest)?);
        }
        Commands::MintToken {
            subject,
            duration,
            scopes,
        } => {
            let o_scopes = if scopes.is_empty() {
                None
            } else {
                Some(scopes)
            };

            println!("🔑 Minting token for '{}'...", subject);

            let token = client.mint_token(&subject, duration, o_scopes).await?;

            println!("✅ SUCCESS! Here is your new token:\n");
            println!("{token}");
            println!("\n(Keep this token safe! It cannot be retrieved again.)");
        }
    }

    Ok(())
}
