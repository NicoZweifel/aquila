//! # AWS Server Example
//!
//! Showcases a server with a [`AwsBatchBackend`] for running compute jobs and a [`S3Storage`] backend server with Presigned URLs enabled.
//!
//! ## Requirements
//!
//! Set the following environment variables:
//! - `AWS_REGION`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` (or use `aws configure`)
//! - `S3_BUCKET`: The name of your bucket.
//! - `BATCH_QUEUE`: The AWS Batch Job Queue ARN or name.
//! - `BATCH_JOB_DEF`: The base Job Definition ARN to use as a template.
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example aws_server --features "server s3 aws mock_auth"
//! ```

use aquila::prelude::*;
use aws_config::BehaviorVersion;
use std::collections::HashMap;
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Config
    let aws_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    let bucket_name = env::var("S3_BUCKET").expect("S3_BUCKET env var required");
    let batch_queue = env::var("BATCH_QUEUE").expect("BATCH_QUEUE env var required");
    let batch_job_def = env::var("BATCH_JOB_DEF").expect("BATCH_JOB_DEF env var required");

    // Add other profiles/job definitions here, e.g., default, deploy, gpu etc.
    let profiles = std::iter::once(batch_job_def).map(|d| ("default", d)).fold(
        HashMap::new(),
        |mut acc, (key, value)| {
            acc.insert(key.to_string(), value.to_string());
            acc
        },
    );

    // Providers & Services
    let storage = S3Storage::new(s3_client, bucket_name)
        .with_prefix("assets/v1/")
        .with_presigning(Duration::from_secs(300));

    // Don't use this in production! This is just for demonstration/testing purposes
    let auth = AllowAllAuth;

    // JWT is not required for this example
    let jwt = NoJwtBackend;

    // Initialize AWS Batch Compute Backend
    let compute = AwsBatchBackend::new(&aws_config, batch_queue, profiles);

    let services = CoreServices {
        storage,
        auth,
        jwt,
        compute,
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
