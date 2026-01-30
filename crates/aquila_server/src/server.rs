use crate::{api, prelude::*};
use aquila_core::prelude::{routes::*, *};
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
            callback: AUTH_CALLBACK.to_string(),
        }
    }
}

impl AquilaServer {
    pub fn build<S: AquilaServices>(self, services: S) -> Router {
        let AquilaServerConfig { callback, .. } = self.config;
        Router::new()
            .route(HEALTH, get(|| async { "OK" }))
            .route(AUTH_LOGIN, get(api::auth_login))
            .route(AUTH_TOKEN, post(api::issue_token))
            .route(callback.as_str(), get(api::auth_callback))
            .route(ASSETS_BY_HASH, get(api::download_asset))
            .route(ASSETS_STREAM_BY_HASH, put(api::upload_asset_stream))
            .route(ASSETS, post(api::upload_asset))
            .route(MANIFEST_BY_VERSION, get(api::get_manifest))
            .route(MANIFEST, post(api::publish_manifest))
            .route(JOBS_RUN, post(api::run))
            .route(JOBS_ATTACH, get(api::attach))
            .layer(DefaultBodyLimit::disable())
            .layer(TraceLayer::new_for_http())
            .with_state(AppState { services })
    }
}
