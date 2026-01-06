//! # 🦅 Aquila
//![![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/NicoZweifel/aquila?tab=readme-ov-file#license)
//![![Crates.io](https://img.shields.io/crates/v/aquila.svg)](https://crates.io/crates/aquila)
//![![Downloads](https://img.shields.io/crates/d/aquila.svg)](https://crates.io/crates/aquila)
//![![Docs](https://docs.rs/aquila/badge.svg)](https://docs.rs/aquila/)
//!
//!> *Your personal flying courier*
//!
//! A modular asset server with support for OAuth and multiple file backends, meant for serving game assets but could probably be used for other things too.
//!
//! This crate serves as an entry point, re-exporting the core logic and
//! optionally including server, client, and storage implementations via feature flags.
//!
//! ## Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | **`server`** | Includes the Axum-based server implementation (`aquila_server`). |
//! | **`client`** | Includes the HTTP client (`aquila_client`) for tooling. |
//! | **`fs`** | Storage backend for the local filesystem (`aquila_fs`). |
//! | **`s3`** | Storage backend for AWS S3 (`aquila_s3`). |
//! | **`opendal`** | Storage backend for OpenDAL (`aquila_opendal`). |
//! | **`github_auth`** | GitHub OAuth2 provider (`aquila_auth_github`). |
//! | **`mock_auth`** | Development authentication provider (`aquila_auth_mock`). |
//!
//! ## Example: Custom Server
//!
//! ```toml
//! [dependencies]
//! aquila = { version = "0.1", features = ["server", "fs", "mock_auth"] }
//! ```
//!
//! ```rust,no_run
//! use aquila::prelude::*;
//!
//! #[tokio::main]
//! async fn main() {
//!     let storage = FileSystemStorage::new("./assets");
//!     let auth = AllowAllAuth;
//!
//!     // Build
//!     let app = AquilaServer::default().build(storage, auth);
//!
//!     // Serve
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```

pub use aquila_core::*;

#[cfg(feature = "server")]
pub mod server {
    pub use aquila_server::*;
}

#[cfg(feature = "client")]
pub mod client {
    pub use aquila_client::*;
}

#[cfg(feature = "fs")]
pub mod fs {
    pub use aquila_fs::*;
}

#[cfg(feature = "mock_auth")]
pub mod auth_mock {
    pub use aquila_auth_mock::*;
}

#[cfg(feature = "s3")]
pub mod s3 {
    pub use aquila_s3::*;
}

#[cfg(feature = "opendal")]
pub mod opendal {
    pub use aquila_opendal::*;
}

#[cfg(feature = "github_auth")]
pub mod auth_github {
    pub use aquila_auth_github::*;
}

pub mod prelude {
    pub use aquila_core::prelude::*;

    #[cfg(feature = "server")]
    pub use aquila_server::prelude::*;

    #[cfg(feature = "client")]
    pub use aquila_client::AquilaClient;

    #[cfg(feature = "fs")]
    pub use aquila_fs::FileSystemStorage;

    #[cfg(feature = "mock_auth")]
    pub use aquila_auth_mock::AllowAllAuth;

    #[cfg(feature = "github_auth")]
    pub use aquila_auth_github::{GithubAuthProvider, GithubConfig};

    #[cfg(feature = "s3")]
    pub use aquila_s3::S3Storage;

    #[cfg(feature = "opendal")]
    pub use aquila_opendal::OpendalStorage;
}
