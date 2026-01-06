use aquila_core::traits::{AuthProvider, StorageBackend};
use axum::extract::DefaultBodyLimit;
use axum::{
    Router,
    routing::{get, post},
};
use std::marker::PhantomData;
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
    login: String,
    callback: String,
    token: String,
}

impl Default for AuthRoutes {
    fn default() -> Self {
        Self {
            login: "/auth/login".to_string(),
            callback: "/auth/callback".to_string(),
            token: "/auth/token".to_string(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AquilaSeverConfig {
    pub jwt_secret: String,
    pub routes: AuthRoutes,
}

impl AquilaServer {
    pub fn build<S: StorageBackend, A: AuthProvider>(self, storage: S, auth: A) -> Router {
        let AquilaSeverConfig {
            jwt_secret, routes, ..
        } = self.config;
        let jwt_service = JwtService::new(&jwt_secret);
        let state = AppState {
            storage,
            auth,
            jwt_service,
        };

        Router::new()
            .route(routes.login.as_str(), get(api::auth_login))
            .route(routes.callback.as_str(), get(api::auth_callback))
            .route(routes.token.as_str(), post(api::issue_token))
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
