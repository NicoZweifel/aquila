//! # Aquila Client
//! [![Crates.io](https://img.shields.io/crates/v/aquila_client.svg)](https://crates.io/crates/aquila_client)
//! [![Downloads](https://img.shields.io/crates/d/aquila_client.svg)](https://crates.io/crates/aquila_client)
//! [![Docs](https://docs.rs/aquila_client/badge.svg)](https://docs.rs/aquila_client/)
//!
//! An async HTTP client for interacting with an Aquila Server.
//!
//! Primarily used by tooling (CLIs, CI/CD scripts, plugins) to upload assets,
//! publish manifests, and mint authentication tokens, as well as to fetch manifests
//! for specific versions.
//!
//! ## Example: Publishing a Manifest
//!
//! ```no_run
//!  use aquila_client::AquilaClient;
//!  use aquila_core::manifest::{AssetManifest, AssetInfo};
//!  use std::path::Path;
//!  use std::collections::HashMap;
//!
//!  async fn run() -> anyhow::Result<()> {
//!     let client = AquilaClient::new("http://localhost:3000", Some("my-token".into()));
//!
//!     // Upload a file
//!     let hash = client.upload_file(Path::new("test.png")).await?;
//!
//!     // Create a manifest entry
//!     let mut assets = HashMap::new();
//!     assets.insert("textures/image.png".into(), AssetInfo {
//!         hash,
//!         size: 1024,
//!         mime_type: Some("image/png".into()),
//!     });
//!
//!     // Publish
//!     let manifest = AssetManifest {
//!         version: "v1.0".into(),
//!         assets,
//!         ..Default::default()
//!     };
//!     client.publish_manifest(&manifest).await?;
//!     Ok(())
//! }
//! ```

use aquila_core::manifest::AssetManifest;
use reqwest::{Client, StatusCode};
use sha2::{Digest, Sha256};
use std::path::Path;

use tokio::fs::File;
use tokio::io::AsyncReadExt;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio_util::io::ReaderStream;

#[derive(Error, Debug)]
pub enum AquilaClientError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Server returned error {0}: {1}")]
    ServerError(StatusCode, String),

    #[error("Validation error: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, AquilaClientError>;

#[derive(Clone)]
pub struct AquilaClient {
    base_url: String,
    client: Client,
    token: Option<String>,
}

#[derive(Serialize)]
struct CreateTokenRequest {
    subject: String,
    duration_seconds: Option<u64>,
    scopes: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct CreateTokenResponse {
    token: String,
    #[allow(dead_code)]
    expires_in: u64,
}

impl AquilaClient {
    pub fn new(base_url: impl Into<String>, token: Option<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
            token,
        }
    }

    fn auth_request(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(token) = &self.token {
            builder.header("Authorization", format!("Bearer {token}"))
        } else {
            builder
        }
    }

    pub async fn fetch_manifest(&self, version: &str) -> Result<AssetManifest> {
        let url = format!("{}/manifest/{version}", self.base_url);
        let response = self.auth_request(self.client.get(&url)).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AquilaClientError::ServerError(status, text));
        }

        let manifest: AssetManifest = response
            .json()
            .await
            .map_err(|e| AquilaClientError::Validation(format!("Failed to parse manifest: {e}")))?;

        Ok(manifest)
    }

    pub async fn mint_token(
        &self,
        subject: &str,
        duration_seconds: Option<u64>,
        scopes: Option<Vec<String>>,
    ) -> Result<String> {
        let url = format!("{}/auth/token", self.base_url);

        let req = CreateTokenRequest {
            subject: subject.to_string(),
            duration_seconds,
            scopes,
        };

        let response = self
            .auth_request(self.client.post(&url))
            .json(&req)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AquilaClientError::ServerError(status, text));
        }

        let data: CreateTokenResponse = response
            .json()
            .await
            .map_err(|_| AquilaClientError::Validation("Failed to parse token response".into()))?;

        Ok(data.token)
    }

    pub async fn upload_file(&self, path: &Path) -> Result<String> {
        let mut file = File::open(path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let mut hasher = Sha256::new();
        hasher.update(&buffer);
        let local_hash = hex::encode(hasher.finalize());

        let url = format!("{}/assets", self.base_url);
        let response = self
            .auth_request(self.client.post(&url))
            .body(buffer)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AquilaClientError::ServerError(status, text));
        }

        let server_hash = response.text().await?;
        if server_hash != local_hash {
            eprintln!("⚠️ Warning: Server hash mismatch");
        }

        Ok(local_hash)
    }

    /// Streams a file. Required for very large files.
    pub async fn upload_stream(&self, path: &Path) -> Result<String> {
        let mut file = File::open(path).await?;
        let mut hasher = Sha256::new();
        // 64KB chunk buffer
        let mut buffer = [0u8; 64 * 1024];

        loop {
            let x = file.read(&mut buffer).await?;
            if x == 0 {
                break;
            }
            hasher.update(&buffer[..x]);
        }

        let local_hash = hex::encode(hasher.finalize());
        let file = File::open(path).await?;
        let size = file.metadata().await?.len();
        let body = reqwest::Body::wrap_stream(ReaderStream::new(file));
        let url = format!("{}/assets/stream/{}", self.base_url, local_hash);

        let response = self
            .auth_request(self.client.put(&url))
            .header("Content-Length", size)
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AquilaClientError::ServerError(status, text));
        }

        Ok(local_hash)
    }

    pub async fn publish_manifest(&self, manifest: &AssetManifest) -> Result<()> {
        let url = format!("{}/manifest", self.base_url);
        let response = self
            .auth_request(self.client.post(&url))
            .json(manifest)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AquilaClientError::ServerError(status, text));
        }

        Ok(())
    }

    pub async fn download_file(&self, hash: &str) -> Result<Vec<u8>> {
        let url = format!("{}/assets/{hash}", self.base_url);
        let response = self.auth_request(self.client.get(&url)).send().await?;
        if !response.status().is_success() {
            return Err(AquilaClientError::ServerError(
                response.status(),
                "Download failed".to_string(),
            ));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}
