//! # OpenDAL Server Example
//!
//! Showcases an [`OpendalStorage`] backend server (configured for fs).
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example opendal_server --features "server opendal mock_auth"
//! ```

use aquila::prelude::*;
use opendal::{Operator, services::Fs};
use std::env;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // This example uses the FileSystem backend, but you can easily swap this
    // for S3, GCS, Azure, etc., by changing the Builder (e.g., opendal::services::S3).
    let mut builder = Fs::default();

    // Config
    let root_path =
        env::var("AQUILA_FS_ROOT").unwrap_or_else(|_| "/tmp/aquila_opendal".to_string());

    builder = builder.root(&root_path);

    let op = Operator::new(builder)
        .expect("Failed to build OpenDAL operator")
        .finish();

    // Providers & Services
    let storage = OpendalStorage::new(op);

    // Don't use this in production! This is just for demonstration/testing purposes
    let auth = AllowAllAuth; // e.g., use GithubAuthProvider or your own instead

    // JWT is not required for this example,
    // see `github_auth_server.rs` for an example using `GithubAuthProvider` and `JwtServiceAuthProvider`.
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

    // Build
    let app = AquilaServer::default().build(services);

    // Serve
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    println!("Server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
