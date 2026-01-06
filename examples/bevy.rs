//! # Bevy Example
//!
//! Loads an asset from the server into Bevy.
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example bevy
//! ```
//!
//! To use a specific token or URL:
//! ```sh
//! AQUILA_URL=http://... AQUILA_TOKEN=... cargo run --example bevy
//! ```

use bevy::prelude::*;
use bevy_aquila::*;

fn main() {
    let token = std::env::var("AQUILA_TOKEN").ok();
    let url = std::env::var("AQUILA_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    App::new()
        .add_plugins(AquilaPlugin::new(AquilaConfig {
            url,
            token,
            version: "v1.0".to_string(),
        }))
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d::default());

    let image = asset_server.load("aquila://test.png");

    commands.spawn(Sprite { image, ..default() });
}
