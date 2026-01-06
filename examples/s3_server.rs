//! # S3 Server Example
//!
//! Showcases a [`S3Storage`] backend server with Presigned URLs enabled.
//!
//! ## Requirements
//!
//! Set the following environment variables:
//! - `AWS_REGION`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` (or use `aws configure`)
//! - `S3_BUCKET`: The name of your bucket.
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example s3_server --features "server s3 mock_auth"
//! ```

use aquila::prelude::*;
use aws_config::BehaviorVersion;
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Config
    let aws_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    let bucket_name = env::var("S3_BUCKET").expect("S3_BUCKET env var required");

    // Providers
    let storage = S3Storage::new(s3_client, bucket_name, Some("assets/v1/".to_string()))
        .with_presigning(Duration::from_secs(300));

    // Don't use this in production! This is just for demonstration/testing purposes
    let auth = AllowAllAuth; // e.g., use GithubAuthProvider instead

    // Build
    let app = AquilaServer::default().build(storage, auth);

    // Serve
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    println!("Server listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
