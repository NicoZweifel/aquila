## Aquila Client
[![Crates.io](https://img.shields.io/crates/v/aquila_client.svg)](https://crates.io/crates/aquila_client)
[![Downloads](https://img.shields.io/crates/d/aquila_client.svg)](https://crates.io/crates/aquila_client)
[![Docs](https://docs.rs/aquila_client/badge.svg)](https://docs.rs/aquila_client/)

An async HTTP client for interacting with an Aquila Server.

Primarily used by tooling (CLIs, CI/CD scripts, plugins) to upload assets,
publish manifests, and mint authentication tokens, as well as to fetch manifests
for specific versions.

### Example: Publishing a Manifest

```rust
 use aquila_client::AquilaClient;
 use aquila_core::manifest::{AssetManifest, AssetInfo};
 use std::path::Path;
 use std::collections::HashMap;

 async fn run() -> anyhow::Result<()> {
    let client = AquilaClient::new("http://localhost:3000", Some("my-token".into()));

    // Upload a file
    let hash = client.upload_file(Path::new("test.png")).await?;

    // Create a manifest entry
    let mut assets = HashMap::new();
    assets.insert("textures/image.png".into(), AssetInfo {
        hash,
        size: 1024,
        mime_type: Some("image/png".into()),
    });

    // Publish
    let manifest = AssetManifest {
        version: "v1.0".into(),
        assets,
        ..Default::default()
    };
    client.publish_manifest(&manifest).await?;
    Ok(())
}
```

License: MIT OR Apache-2.0
