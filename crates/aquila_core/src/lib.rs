//! # Aquila Core
//! [![Crates.io](https://img.shields.io/crates/v/aquila_core.svg)](https://crates.io/crates/aquila_core)
//! [![Downloads](https://img.shields.io/crates/d/aquila_core.svg)](https://crates.io/crates/aquila_core)
//! [![Docs](https://docs.rs/aquila_core/badge.svg)](https://docs.rs/aquila_core/)
//!
//! Types and traits for the ecosystem.
//!
//! Defines the protocol used by clients and servers.
//!
//! - **[`AssetManifest`](asset::AssetManifest)**: The source of truth for a game version. Maps logical paths (e.g., `textures/test.png`) to physical content hashes.
//! - **[`StorageBackend`](traits::StorageBackend)**: Trait for implementing storage layers (e.g., S3, Filesystem).
//! - **[`AuthProvider`](traits::AuthProvider)**: Trait for implementing user verification strategies.

pub mod asset;
pub mod claims;
pub mod error;
pub mod job;
pub mod traits;

pub mod prelude {
    pub use super::asset::*;
    pub use super::claims::*;
    pub use super::error::*;
    pub use super::job::*;
    pub use super::traits::*;
}
