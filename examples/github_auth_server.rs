//! # GitHub Auth Server
//!
//! Showcases [`GithubAuthProvider`] backend server with local filesystem storage.
//!
//! ## Requirements
//!
//! Set the following environment variables:
//! - `AQUILA_JWT_SECRET`
//! - `GITHUB_CLIENT_ID`
//! - `GITHUB_CLIENT_SECRET`
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example github_auth_server --features "server fs github_auth"
//! ```

use aquila::prelude::*;
use std::env;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Config
    let required_org = env::var("AQUILA_GITHUB_ORG").ok();

    // In Production this should be a long, random string generated and set by you.
    // For this example, fall back to "TOP_SECRET" (the default) if none is provided.
    let jwt_secret = env::var("AQUILA_JWT_SECRET").unwrap_or("TOP_SECRET".to_string());

    // Must match the callback route in the GitHub app and the server config callback, see below.
    let redirect_uri = "http://localhost:3000/auth/callback".to_string();
    let gh_cfg = env::var("GITHUB_CLIENT_ID")
        .and_then(|client_id| {
            env::var("GITHUB_CLIENT_SECRET").map(|client_secret| GithubConfig {
                redirect_uri,
                client_id,
                client_secret,
                required_org,
            })
        })
        .ok();

    // Providers
    let storage = FileSystemStorage::new("./aquila_data");
    let gh_auth = GithubAuthProvider::new(gh_cfg);
    let jwt_service = JwtService::new(&jwt_secret);
    let auth = JWTServiceAuthProvider::new(jwt_service, gh_auth);

    // Build
    let app = AquilaServer::new(AquilaSeverConfig {
        jwt_secret,
        // this is the default but just to be explicit, see above.
        callback: "/auth/callback".to_string(),
    })
    .build(storage, auth);

    // Serve
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    println!("Server listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
