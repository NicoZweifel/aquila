//! # Bevy Aquila
//! [![Crates.io](https://img.shields.io/crates/v/bevy_aquila.svg)](https://crates.io/crates/bevy_aquila)
//! [![Downloads](https://img.shields.io/crates/d/bevy_aquila.svg)](https://crates.io/crates/bevy_aquila)
//! [![Docs](https://docs.rs/bevy_aquila/badge.svg)](https://docs.rs/bevy_aquila/)
//!
//! [Aquila](https://github.com/NicoZweifel/aquila) integration for the Bevy game engine.
//!
//! This plugin registers a custom `AssetReader` for the `aquila://` scheme. When a file is requested,
//! the plugin:
//! 1. Fetches the `AssetManifest` for the configured version (lazily cached).
//! 2. Resolves the logical path to a content hash.
//! 3. Downloads the binary blob from the server.
//!
//! ## Usage
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_aquila::{AquilaPlugin, AquilaConfig};
//!
//! App::new()
//!     .add_plugins(AquilaPlugin::new(AquilaConfig {
//!         url: "http://localhost:3000".to_string(),
//!         version: "v1.0".to_string(),
//!         token: None,
//!     }));
//! ```
//! ## Compatibility
//!
//! | bevy | bevy_aquila |
//! |---|---|
//! | 0.17 | 0.6 |
//!

use aquila_client::{AquilaClient, AquilaClientError};
use aquila_core::manifest::AssetManifest;
use bevy_app::prelude::*;
use bevy_asset::AssetApp;
use bevy_asset::io::{
    AssetReader, AssetReaderError, AssetSourceBuilder, AssetSourceId, PathStream, Reader, VecReader,
};
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use std::path::Path;
use std::sync::Arc;
use tokio::{runtime, sync::OnceCell};
use tracing::{error, info, warn};

/// Configuration for the Aquila Plugin
#[derive(Resource, Clone, Reflect, Debug)]
#[reflect(Resource, Clone, Debug)]
pub struct AquilaConfig {
    /// The base URL e.g. "http://localhost:3000"
    pub url: String,
    /// The JWT Token for authentication
    pub token: Option<String>,
    /// The game version to load e.g. "v1.0"
    pub version: String,
}

impl Default for AquilaConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:3000".to_string(),
            token: None,
            version: "latest".to_string(),
        }
    }
}

pub struct AquilaPlugin;

impl Plugin for AquilaPlugin {
    fn build(&self, app: &mut App) {
        let cfg = app
            .world()
            .get_resource::<AquilaConfig>()
            .cloned()
            .expect("AquilaConfig must be inserted before adding AquilaPlugin");

        app.register_asset_source(
            AssetSourceId::Name("aquila".into()),
            AssetSourceBuilder::default()
                .with_reader(move || Box::new(AquilaAssetReader::new(cfg.clone()))),
        );
    }
}

struct AquilaAssetReader {
    client: AquilaClient,
    target_version: String,
    /// Lazy-loaded manifest
    manifest: Arc<OnceCell<AssetManifest>>,
    runtime: Arc<runtime::Runtime>,
}

impl AquilaAssetReader {
    fn new(config: AquilaConfig) -> Self {
        let runtime = runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime for AquilaAssetReader");

        Self {
            client: AquilaClient::new(config.url, config.token),
            target_version: config.version,
            manifest: Arc::new(OnceCell::new()),
            runtime: Arc::new(runtime),
        }
    }

    /// Fetch and cache the Manifest
    async fn get_manifest(&self) -> Result<&AssetManifest, AssetReaderError> {
        self.manifest
            .get_or_try_init(|| async {
                info!(
                    "Fetching Aquila Manifest for version: {}",
                    self.target_version
                );
                let client = self.client.clone();
                let version = self.target_version.clone();
                let runtime = self.runtime.clone();

                runtime
                    .spawn(async move { client.fetch_manifest(&version).await })
                    .await
                    .map_err(|join_err| {
                        AssetReaderError::Io(Arc::from(std::io::Error::other(join_err)))
                    })?
                    .map_err(|e| {
                        error!("Manifest fetch failed: {}", e);
                        AssetReaderError::Io(Arc::from(std::io::Error::other(e)))
                    })
            })
            .await
    }

    async fn resolve_hash(&self, path: &Path) -> Result<String, AssetReaderError> {
        let manifest = self.get_manifest().await?;
        let path_str = path.to_string_lossy().replace('\\', "/");

        if let Some(info) = manifest.assets.get(&path_str) {
            Ok(info.hash.clone())
        } else {
            warn!("Asset not found in manifest: {}", path_str);
            Err(AssetReaderError::NotFound(path.to_path_buf()))
        }
    }
}

impl AssetReader for AquilaAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let hash = self.resolve_hash(path).await?;
        let client = self.client.clone();
        let runtime = self.runtime.clone();

        let bytes = runtime
            .spawn(async move { client.download_file(&hash).await })
            .await
            .map_err(|join_err| {
                AssetReaderError::Io(Arc::from(std::io::Error::other(format!(
                    "Tokio join error: {}",
                    join_err
                ))))
            })?
            .map_err(|e| match e {
                AquilaClientError::ServerError(c, _) if c.as_u16() == 404 => {
                    AssetReaderError::NotFound(path.to_path_buf())
                }
                _ => AssetReaderError::Io(Arc::from(std::io::Error::other(e))),
            })?;

        Ok(VecReader::new(bytes))
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let res: Result<VecReader, AssetReaderError> =
            Err(AssetReaderError::NotFound(path.to_path_buf()));
        res
    }

    async fn read_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        Ok(Box::new(futures_lite::stream::empty()))
    }

    async fn is_directory<'a>(&'a self, _path: &'a Path) -> Result<bool, AssetReaderError> {
        Ok(false)
    }
}
