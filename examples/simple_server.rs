//! # Simple Server Example
//!
//! Showcases a minimal [`AquilaServer`] using the local filesystem and mock authentication.
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

    // Providers
    let storage = FileSystemStorage::new("./aquila_data");

    // Don't use this in production! This is just for demonstration/testing purposes
    let auth = AllowAllAuth; // e.g., use GithubAuthProvider instead

    // Build App
    let app = AquilaServer::default().build(storage, auth);

    // Serve
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    println!("Server listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
