use crate::api::{asset, auth, job, manifest};
use crate::prelude::*;
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
            .route(AUTH_LOGIN, get(auth::login))
            .route(AUTH_TOKEN, post(auth::issue_token))
            .route(callback.as_str(), get(auth::callback))
            .route(ASSETS_BY_HASH, get(asset::download))
            .route(ASSETS_STREAM_BY_HASH, put(asset::upload))
            .route(ASSETS, post(asset::upload_asset))
            .route(MANIFEST_BY_VERSION, get(manifest::get))
            .route(MANIFEST, post(manifest::publish))
            .route(JOBS_RUN, post(job::run))
            .route(JOBS_ATTACH, get(job::attach))
            .route(JOBS_STOP, post(job::stop))
            .route(JOBS_LOGS, get(job::get_logs))
            .route(JOBS_STATUS, get(job::get_status))
            .layer(DefaultBodyLimit::disable())
            .layer(TraceLayer::new_for_http())
            .with_state(AppState { services })
    }
}
