use aquila_core::traits::{AuthProvider, StorageBackend};
use axum::extract::DefaultBodyLimit;
use axum::{
    Router,
    routing::{get, post},
};
use tower_http::trace::TraceLayer;

mod api;

pub mod jwt;

pub mod auth;
pub mod state;

use crate::jwt::JwtService;
use state::AppState;

/// The builder for the Aquila Server.
pub struct AquilaServer {
    config: AquilaSeverConfig,
}

impl Default for AquilaServer {
    fn default() -> Self {
        Self {
            config: Default::default(),
        }
    }
}

impl AquilaServer {
    pub fn new(config: AquilaSeverConfig) -> Self {
        Self { config }
    }
}

#[derive(Clone, Debug)]
pub struct AuthRoutes {
    callback: String,
}

impl Default for AuthRoutes {
    fn default() -> Self {
        Self {
            callback: "/auth/callback".to_string(),
        }
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
            .route("/auth/login", get(api::auth_login))
            .route(callback.as_str(), get(api::auth_callback))
            .route("/auth/token", post(api::issue_token))
            .route("/assets/{hash}", get(api::download_asset))
            .route("/assets", post(api::upload_asset))
            .route("/manifest/{version}", get(api::get_manifest))
            .route("/manifest", post(api::publish_manifest))
            .layer(DefaultBodyLimit::disable())
            .layer(TraceLayer::new_for_http())
            .with_state(state)
    }
}

pub mod prelude {
    pub use crate::AquilaServer;
    pub use crate::auth::*;
    pub use crate::jwt::*;
    pub use crate::state::*;
}
