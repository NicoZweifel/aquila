## Bevy Aquila
[![Crates.io](https://img.shields.io/crates/v/bevy_aquila.svg)](https://crates.io/crates/bevy_aquila)
[![Downloads](https://img.shields.io/crates/d/bevy_aquila.svg)](https://crates.io/crates/bevy_aquila)
[![Docs](https://docs.rs/bevy_aquila/badge.svg)](https://docs.rs/bevy_aquila/)

[Aquila](https://github.com/NicoZweifel/aquila) integration for the Bevy game engine.

This plugin registers a custom `AssetReader` for the `aquila://` scheme. When a file is requested,
the plugin:
1. Fetches the `AssetManifest` for the configured version (lazily cached).
2. Resolves the logical path to a content hash.
3. Downloads the binary blob from the server.

### Usage

```rust
use bevy::prelude::*;
use bevy_aquila::{AquilaPlugin, AquilaConfig};

App::new()
    .add_plugins(AquilaPlugin::new(AquilaConfig {
        url: "http://localhost:3000".to_string(),
        version: "v1.0".to_string(),
        token: None,
    }));
```
### Compatibility

| bevy | bevy_aquila |
|---|---|
| 0.17 | 0.3 |


License: MIT OR Apache-2.0
