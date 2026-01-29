use crate::{api, prelude::*};
use aquila_core::prelude::*;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post, put},
};
use tower_http::trace::TraceLayer;

/// The builder for the Aquila Server.
#[derive(Clone, Debug, Default)]
pub struct AquilaServer {
    config: AquilaServerConfig,
}

impl AquilaServer {
    pub fn new(config: AquilaServerConfig) -> Self {
        Self { config }
    }
}

#[derive(Clone, Debug)]
pub struct AquilaServerConfig {
    /// The callback URL for the auth provider.
    ///
    /// Defaults to `/auth/callback`.
    pub callback: String,
}

impl Default for AquilaServerConfig {
    fn default() -> Self {
        Self {
            callback: "/auth/callback".to_string(),
        }
    }
}

impl AquilaServer {
    pub fn build<S: AquilaServices>(self, services: S) -> Router {
        let AquilaServerConfig { callback, .. } = self.config;
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
            .route("/jobs/run", post(api::run))
            .route("/jobs/{id}/attach", get(api::attach))
            .layer(DefaultBodyLimit::disable())
            .layer(TraceLayer::new_for_http())
            .with_state(AppState { services })
    }
}
