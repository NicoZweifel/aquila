## Aquila S3 Storage
[![Crates.io](https://img.shields.io/crates/v/aquila_s3.svg)](https://crates.io/crates/aquila_s3)
[![Downloads](https://img.shields.io/crates/d/aquila_s3.svg)](https://crates.io/crates/aquila_s3)
[![Docs](https://docs.rs/aquila_s3/badge.svg)](https://docs.rs/aquila_s3/)

AWS S3 backend integration for Aquila.

Uses the official [`aws-sdk-s3`] to store assets in an S3 bucket. It supports
prefixes for organizing data within shared buckets and **Presigned URLs** for
downloads via S3/CDN directly.

### Configuration

Requires the standard AWS environment variables (e.g., `AWS_REGION`, `AWS_ACCESS_KEY_ID`)
handled by `aws-config`.

### Usage

```rust
let config = aws_config::load_from_env().await;
let client = Client::new(&config);

let storage = S3Storage::new(
    client,
    "my-game-assets".to_string(), // Bucket
    Some("production/".to_string()) // Optional Prefix
)
// Optional: Enable Presigned URLs (Direct S3 Download)
.with_presigning(Duration::from_secs(300));
```

License: MIT OR Apache-2.0
