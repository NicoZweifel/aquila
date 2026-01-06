//! # Aquila Core
//!
//! Types and traits for the Aquila asset server ecosystem.
//!
//! Defines the protocol used by clients and servers to communicate asset metadata.
//!
//! - **[`AssetManifest`](manifest::AssetManifest)**: The source of truth for a game version. Maps logical paths (e.g., `textures/test.png`) to physical content hashes.
//! - **[`StorageBackend`](traits::StorageBackend)**: Trait for implementing storage layers (e.g., S3, Filesystem).
//! - **[`AuthProvider`](traits::AuthProvider)**: Trait for implementing user verification strategies.

pub mod error;
pub mod manifest;
pub mod traits;

pub mod prelude {
    pub use super::error::*;
    pub use super::manifest::*;
    pub use super::traits::*;
}
