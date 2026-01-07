use crate::{api, prelude::*};
use aquila_core::prelude::*;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post, put},
};
use tower_http::trace::TraceLayer;
use tracing::warn;

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

const DEFAULT_SECRET: &str = "TOP_SECRET";

impl Default for AquilaSeverConfig {
    fn default() -> Self {
        Self {
            jwt_secret: DEFAULT_SECRET.to_string(),
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
        if jwt_secret == DEFAULT_SECRET {
            warn!("Default JWT secret used. Consider setting `jwt_secret` to a secure value!")
        }
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
