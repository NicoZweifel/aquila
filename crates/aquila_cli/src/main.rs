//! # Aquila CLI
//! [![Crates.io](https://img.shields.io/crates/v/aquila_cli.svg)](https://crates.io/crates/aquila_cli)
//! [![Downloads](https://img.shields.io/crates/d/aquila_cli.svg)](https://crates.io/crates/aquila_cli)
//!
//! A command-line interface for managing the server.
//!
//! Allows developers and CI/CD pipelines to upload assets, publish versions and manage tokens.
//!
//! **NOTE**: This tool requires a running server.
//!
//! ## Installation
//!
//! crates.io:
//! ```bash
//! cargo install aquila_cli
//! ```
//! From source:
//! ```bash
//! cargo install --path crates/aquila_cli
//! ```
//!
//! ## Configuration
//!
//! Can be configured via flags or environment variables:
//!
//! * **URL**: `--url` or `AQUILA_URL` (default: `http://localhost:3000`)
//! * **Token**: `--token` or `AQUILA_TOKEN`
//!
//! ## Common Commands
//!
//! * **Publish a version**:
//!     ```bash
//!     aquila publish ./assets --version v1.0.0
//!     ```
//! * **Mint a long-lived token** (requires admin/write permissions):
//!     ```bash
//!     aquila mint-token  "build_server" --duration 31536000
//!     ```
//! * **Generate a JWT Secret** (for server setup):
//!     ```bash
//!     aquila generate-secret
//!     ```

use aquila_client::AquilaClient;
use aquila_core::prelude::*;

use chrono::Utc;
use clap::{Parser, Subcommand};
use rand::Rng;
use rand::distr::Alphanumeric;
use walkdir::WalkDir;

use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "aquila")]
#[command(about = "CLI for Aquila Asset Server")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Server URL
    #[arg(
        short,
        long,
        default_value = "http://localhost:3000",
        env = "AQUILA_URL"
    )]
    url: String,

    #[arg(short, long, env = "AQUILA_TOKEN")]
    token: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Upload a single file
    Upload {
        path: PathBuf,
        #[arg(short, long)]
        /// Use streaming upload (recommended for very large files)
        stream: bool,
    },
    /// Publish a directory as a new Game Version
    Publish {
        /// The directory containing assets (e.g., "./assets")
        dir: PathBuf,

        /// The version string (e.g., "0.1.0" or git sha)
        #[arg(short, long)]
        version: String,

        /// Use streaming upload (recommended for very large files)
        #[arg(short, long)]
        stream: bool,

        /// Don't update the 'latest' version tag.
        /// Use this when publishing patches for older versions.
        #[arg(short, long)]
        no_latest: bool,
    },
    /// Download a file by hash
    Download {
        hash: String,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Fetch and display a manifest for a specific version
    Manifest { version: String },
    /// Compute job commands (run, attach).
    Job {
        #[command(subcommand)]
        command: ComputeCommands,
    },
    /// Start a login flow, if supported by the server.
    Login,
    /// Generate a JWT Secret for use with the server.
    GenerateSecret,
    /// Mint a long-lived token for a specific subject, e.g., a game client or build pipeline.
    Mint {
        /// The subject name (e.g. "game_client_v1")
        subject: String,

        /// Duration in seconds (default: 1 year)
        #[arg(short, long)]
        duration: Option<u64>,

        /// Optional scopes (comma separated, e.g. "read,write")
        #[arg(short = 'S', long, value_delimiter = ',', default_value = "read")]
        scopes: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ComputeCommands {
    /// Run a new compute job
    Run {
        /// Image to run, can be [`None`] if the backend uses a base image.
        #[arg(short, long)]
        img: Option<String>,

        /// The queue to use for the job.
        ///
        /// [`None`] if the [`ComputeBackend`] doesn't support it, or if using the default queue.
        #[arg(short, long)]
        queue: Option<String>,

        /// Command to execute
        #[arg(last = true)]
        cmd: Vec<String>,

        /// Attach immediately after starting
        #[arg(short, long)]
        attach: bool,

        /// GPU driver to use. If [`None`], no gpu is attached.
        #[arg(short, long)]
        gpu: Option<String>,

        /// Remove the job/container after it finishes.
        #[arg(short, long)]
        remove: bool,

        /// Environment variables (e.g. --env REPO_URL=... --env FOO=bar)
        #[arg(short, long, value_name = "KEY=VALUE")]
        env: Vec<String>,
    },

    /// Attach to a running compute job
    Attach {
        /// Job ID
        id: String,
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
        Commands::Upload { path, stream } => {
            let hash = if stream {
                println!("📤  Streaming...");
                client.upload_stream(&path).await?
            } else {
                println!("📤 Uploading blob...");
                client.upload_file(&path).await?
            };
            println!("✅ Upload successful! Hash: {hash}");
        }
        Commands::Publish {
            dir,
            version,
            stream,
            no_latest,
        } => {
            println!("🚀 Publishing version '{version}' from {dir:?}...");
            if stream {
                println!("ℹ️ Using streaming upload mode");
            }

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

                let hash = if stream {
                    client.upload_stream(path).await?
                } else {
                    client.upload_file(path).await?
                };

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

            let latest = !no_latest;
            client.publish_manifest(&manifest, latest).await?;

            println!("✅ Successfully published version {version} with {count} assets.",);
            if latest {
                println!("🏷️ Tagged as 'latest'.");
            } else {
                println!("ℹ️ Skipped 'latest' tag update.");
            }
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
        Commands::Manifest { version } => {
            println!("🔍 Fetching manifest for version '{}'...", version);
            let manifest = client.fetch_manifest(&version).await?;
            println!("{}", serde_json::to_string_pretty(&manifest)?);
        }
        Commands::Job { command } => match command {
            ComputeCommands::Run {
                img,
                queue,
                cmd,
                attach,
                gpu,
                remove,
                env,
            } => {
                let env: Vec<(String, String)> = env
                    .into_iter()
                    .filter_map(|s| {
                        s.split_once('=')
                            .map(|(k, v)| (k.to_string(), v.to_string()))
                    })
                    .collect();

                let req = JobRequest {
                    img,
                    queue,
                    cmd,
                    gpu,
                    remove,
                    env,
                    ..Default::default()
                };

                let res = client.run(req).await?;
                println!("⚙️ Job started: {} {:?}", res.id, res.status);

                if attach {
                    println!("🪝 Attaching...");
                    if let Err(e) = client.attach(&res.id).await {
                        eprintln!("❌ Attach interrupted: {}", e);
                    }
                    println!("✅ Job finished!");
                }
            }

            ComputeCommands::Attach { id } => {
                println!("🪝 Attaching to job {}...", id);
                client.attach(&id).await?;
            }
        },
        Commands::Mint {
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
