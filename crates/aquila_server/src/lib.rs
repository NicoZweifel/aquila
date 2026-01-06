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
//! * **`admin`**: Full access. (Note: Admin tokens cannot be minted via the API and only Admins can mint tokens).
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
pub mod state;

use aquila_core::traits::{AuthProvider, StorageBackend};
use axum::extract::DefaultBodyLimit;
use axum::routing::put;
use axum::{
    Router,
    routing::{get, post},
};
use jwt::JwtService;
use state::AppState;
use tower_http::trace::TraceLayer;

/// The builder for the Aquila Server.
#[derive(Clone, Debug, Default)]
pub struct AquilaServer {
    config: AquilaSeverConfig,
}

impl AquilaServer {
    pub fn new(config: AquilaSeverConfig) -> Self {
        Self { config }
    }
}

#[derive(Clone, Debug)]
pub struct AquilaSeverConfig {
    pub jwt_secret: String,
    pub callback: String,
}

impl Default for AquilaSeverConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "TOP_SECRET".to_string(),
            callback: "/auth/callback".to_string(),
        }
    }
}

impl AquilaServer {
    pub fn build<S: StorageBackend, A: AuthProvider>(self, storage: S, auth: A) -> Router {
        let AquilaSeverConfig {
            jwt_secret,
            callback,
            ..
        } = self.config;
        let jwt_service = JwtService::new(&jwt_secret);
        let state = AppState {
            storage,
            auth,
            jwt_service,
        };

        Router::new()
            .route("/health", get(|| async { "OK" }))
            .route("/auth/login", get(api::auth_login))
            .route("/auth/token", post(api::issue_token))
            .route(callback.as_str(), get(api::auth_callback))
            .route("/assets/{hash}", get(api::download_asset))
            .route("/assets/stream/{hash}", put(api::upload_asset_stream))
            .route("/assets", post(api::upload_asset))
            .route("/manifest/{version}", get(api::get_manifest))
            .route("/manifest", post(api::publish_manifest))
            .layer(DefaultBodyLimit::disable())
            .layer(TraceLayer::new_for_http())
            .with_state(state)
    }
}

pub mod prelude {
    pub use crate::auth::*;
    pub use crate::jwt::*;
    pub use crate::state::*;
    pub use crate::{AquilaServer, AquilaSeverConfig};
}
