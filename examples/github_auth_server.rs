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

    // Auth
    //
    // In Production this should be a long, random string generated and set by you.
    // You can generate one with the cli or with `openssl rand -base64 32`.
    //
    // For this example, fall back to "TOP_SECRET" if none is provided.
    let jwt_secret = env::var("AQUILA_JWT_SECRET").unwrap_or("TOP_SECRET".to_string());

    // Config
    // If no org is defined, any GitHub user can sign in. You probably should panic if this is empty in production.
    // For the purposes of this example we'll leave it optional.
    let required_org = env::var("AQUILA_GITHUB_ORG").ok();
    // Must match the callback route in the GitHub app and the server config callback, see below.
    let redirect_uri = "http://localhost:3000/auth/callback".to_string();
    let gh_cfg = env::var("GITHUB_CLIENT_ID")
        .and_then(|client_id| {
            env::var("GITHUB_CLIENT_SECRET").map(|client_secret| GithubConfig {
                redirect_uri,
                client_id,
                client_secret,
                required_org,
                // All users will get these by default, these can be managed by the `PermissionService`.
                // If you are not defining a required organization in the configuration above,
                // you should consider passing an empty vec by default and using the `PermissionService` to grant certain users permissions.
                default_scopes: vec!["read".to_string(), "write".to_string()],
            })
        })
        .ok();

    // Providers & Services
    let storage = FileSystemStorage::new("./aquila_data");
    let gh_auth = GithubAuthProvider::new(gh_cfg);
    let jwt = JwtService::new(&jwt_secret);
    let auth = JWTServiceAuthProvider::new(jwt.clone(), gh_auth);
    let compute = NoComputeBackend;
    let permissions = StandardPermissionService;

    let services = CoreServices {
        storage,
        auth,
        jwt,
        compute,
        permissions,
    };

    // Build
    let app = AquilaServer::new(AquilaServerConfig {
        // this is the default but just to be explicit, see above.
        callback: "/auth/callback".to_string(),
    })
    .build(services);

    // Serve
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    println!("Server listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
