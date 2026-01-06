//! # Simple Client Example
//!
//! Demonstrates uploading a file, creating a manifest, and publishing a version.
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example simple_client --features "client"
//! ```

use aquila::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = AquilaClient::new("http://localhost:3000", Default::default());
    let file_path = Path::new("assets/test.png");
    let hash = client.upload_file(file_path).await?;
    let file_size = fs::metadata(file_path).await?.len();

    let mut assets = HashMap::new();
    assets.insert(
        "textures/test.png".to_string(),
        AssetInfo {
            hash: hash.clone(),
            size: file_size,
            mime_type: Some("image/png".to_string()),
        },
    );

    let version = "v1.0";
    let manifest = AssetManifest {
        version: version.to_string(),
        published_at: chrono::Utc::now(),
        published_by: "simple_client_example".to_string(),
        assets,
    };

    // Publish the Manifest
    client.publish_manifest(&manifest).await?;

    // Download the image to a different location
    let output_path = Path::new("downloaded_test.png");

    let data = client.download_file(&hash).await?;
    fs::write(output_path, data).await?;

    println!("Image downloaded to download_test.png!");

    Ok(())
}
