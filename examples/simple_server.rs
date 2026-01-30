//! # Simple Server Example
//!
//! Showcases a minimal [`AquilaServer`] using the local filesystem, no compute backend to run jobs and mock authentication.
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example simple_server --features "server fs mock_auth"
//! ```

use aquila::prelude::*;
use std::env;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Providers & Services
    let storage = FileSystemStorage::new("./aquila_data");

    // Don't use this in production! This is just for demonstration/testing purposes
    let auth = AllowAllAuth; // e.g., use GithubAuthProvider or your own instead

    // JWT is not required for this example, see `github_auth_server.rs` for an example using `GithubAuthProvider` and `JwtServiceAuthProvider`.
    let jwt = NoJwtBackend;

    // Compute is not required for this example, see `docker_server.rs` for an example using `DockerComputeBackend`.
    let compute = NoComputeBackend;

    // No Permissions Service required for this example,
    // see `github_auth_server.rs` for an example using [`StandardPermissionsService`] to map scopes.
    let permissions = NoPermissionService;

    let services = CoreServices {
        storage,
        auth,
        jwt,
        compute,
        permissions,
    };

    // Build App
    let app = AquilaServer::default().build(services);

    // Serve
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    println!("Server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
