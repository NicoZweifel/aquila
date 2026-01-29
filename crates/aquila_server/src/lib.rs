//! # Aquila Server
//![![Crates.io](https://img.shields.io/crates/v/aquila_server.svg)](https://crates.io/crates/aquila_server)
//![![Downloads](https://img.shields.io/crates/d/aquila_server.svg)](https://crates.io/crates/bevy_aquila)
//![![Docs](https://docs.rs/aquila_server/badge.svg)](https://docs.rs/aquila_server/)
//!
//! A modular, Axum-based asset server implementation.
//!
//! Provides the [`AquilaServer`] builder, which ties together a storage backend and an authentication provider
//! to serve assets.
//!
//! ## Permissions
//!
//! Enforces a scoped permission system. Authentication providers must grant
//! the following scopes in their `User` object:
//!
//! * **`read`**: to download assets, fetch manifests.
//! * **`write`**: to upload assets, publish manifests.
//! * **`admin`**: Full access. (Note: admin/write tokens cannot be minted via the API and only write access can mint tokens).
//!
//! ## Example
//!
//! ```no_run
//! use aquila_server::prelude::*;
//! use aquila_fs::FileSystemStorage;
//! use aquila_auth_mock::AllowAllAuth;
//!
//! # async fn run() {
//! let storage = FileSystemStorage::new("./assets");
//! let auth = AllowAllAuth;
//!
//! let app = AquilaServer::default().build(storage, auth);
//! # }
//! ```

mod api;

pub mod jwt;

pub mod auth;
pub mod server;
pub mod services;
pub mod state;

pub mod prelude {
    pub use crate::auth::*;
    pub use crate::jwt::*;
    pub use crate::server::*;
    pub use crate::services::*;
    pub use crate::state::*;
}
