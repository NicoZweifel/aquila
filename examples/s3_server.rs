use aquila::prelude::*;
use aws_config::BehaviorVersion;
use std::env;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Config
    let aws_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    let bucket_name = env::var("S3_BUCKET").expect("S3_BUCKET env var required");

    // Providers
    let storage = S3Storage::new(s3_client, bucket_name, Some("assets/v1/".to_string()));

    // Don't use this in production! This is just for demonstration/testing purposes
    let auth = AllowAllAuth; // e.g., use GithubAuthProvider instead

    // Build
    let app = AquilaServer::default()
    .build(storage, auth);

    // Serve
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    println!("Server listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
