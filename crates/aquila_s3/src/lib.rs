//! # Aquila S3 Storage
//! [![Crates.io](https://img.shields.io/crates/v/aquila_s3.svg)](https://crates.io/crates/aquila_s3)
//! [![Downloads](https://img.shields.io/crates/d/aquila_s3.svg)](https://crates.io/crates/aquila_s3)
//! [![Docs](https://docs.rs/aquila_s3/badge.svg)](https://docs.rs/aquila_s3/)
//!
//! A storage backend powered by [AWS S3](https://aws.amazon.com/s3/).
//!
//! Uses the official [`aws-sdk-s3`] to store assets in an S3 bucket. It supports
//! prefixes for organizing data within shared buckets and **Presigned URLs** for
//! downloads via S3/CDN directly.
//!
//! ## Configuration
//!
//! Requires the standard AWS environment variables (e.g., `AWS_REGION`, `AWS_ACCESS_KEY_ID`)
//! handled by `aws-config`.
//!
//! ## Usage
//!
//! ```no_run
//! # use aquila_s3::S3Storage;
//! # use aws_sdk_s3::Client;
//! # use std::time::Duration;
//! # async fn run() {
//! let config = aws_config::load_from_env().await;
//! let client = Client::new(&config);
//!
//! let storage = S3Storage::new(
//!     client,
//!     "my-game-assets".to_string(), // Bucket
//!     Some("production/".to_string()) // Optional Prefix
//! )
//! // Optional: Enable Presigned URLs (Direct S3 Download)
//! .with_presigning(Duration::from_secs(300));
//! # }
//! ```

use aquila_core::prelude::*;
use aws_sdk_s3::Client;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::{ByteStream, SdkBody};
use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http_body_util::StreamBody;
use hyper::body::Frame;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tracing::{debug, error, instrument};

#[derive(Clone)]
pub struct S3Storage {
    client: Client,
    bucket: String,
    prefix: String,
    /// If set, generate presigned URLs for this duration.
    presign_duration: Option<Duration>,
}

/// A wrapper to assert Sync for a Stream that is [`Send`].
///
/// Required because `aws-sdk-s3` (via `SdkBody`) requires the body to be [`Sync`],
/// but Axum streams are generally `!Sync`. Since the data is streamed
/// linearly and ownership is moved into the SDK client, [`Sync`] can safely be asserted.
struct SyncStream<S>(S);

// SAFETY: Strictly wrap `Send` stream. The usage pattern in SdkBody/hyper involves
// polling via `&mut self`, which enforces exclusive access.
unsafe impl<S: Send> Sync for SyncStream<S> {}

impl<S, T, E> Stream for SyncStream<S>
where
    S: Stream<Item = Result<T, E>> + Unpin,
{
    type Item = Result<T, E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.0).poll_next(cx)
    }
}

impl S3Storage {
    pub fn new(client: Client, bucket: String, prefix: Option<String>) -> Self {
        Self {
            client,
            bucket,
            prefix: prefix.unwrap_or_default(),
            presign_duration: None,
        }
    }

    /// Enable presigned URLs (e.g. 5 minutes)
    pub fn with_presigning(mut self, duration: Duration) -> Self {
        self.presign_duration = Some(duration);
        self
    }

    /// Private helper to create a key from a path. Adds the prefix if set.
    fn key(&self, path: &str) -> String {
        self.prefix
            .is_empty()
            .then(|| path.to_string())
            .unwrap_or(format!("{}{path}", self.prefix))
    }

    /// Private helper to check existence.
    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let res = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await;

        match res {
            Ok(_) => {
                debug!("Blob already exists in S3");
                Ok(true)
            }
            Err(SdkError::ServiceError(err)) if err.err().is_not_found() => Ok(false),
            Err(err) => Err(StorageError::Generic(format!(
                "S3 Head Object Error: {err:?}"
            ))),
        }
    }
}

impl StorageBackend for S3Storage {
    #[instrument(skip(self, data), fields(bucket = %self.bucket, key))]
    async fn write_blob(&self, hash: &str, data: Bytes) -> Result<bool, StorageError> {
        let key = self.key(hash);
        tracing::Span::current().record("key", &key);

        if self.exists(&key).await? {
            return Ok(false);
        }

        debug!("Uploading new blob to S3...");
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(data))
            .send()
            .await
            .map_err(|e| {
                error!("Failed to upload blob: {e:?}");
                StorageError::Generic(format!("S3 Upload Error: {e:?}"))
            })?;

        debug!("Upload successful");
        Ok(true)
    }

    #[instrument(skip(self, stream), fields(bucket = %self.bucket, key))]
    async fn write_stream(
        &self,
        hash: &str,
        stream: Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>,
        content_length: Option<u64>,
    ) -> Result<bool, StorageError> {
        let key = self.key(hash);
        tracing::Span::current().record("key", &key);

        if self.exists(&key).await? {
            return Ok(false);
        }

        debug!("Streaming upload to S3...");
        let byte_stream = ByteStream::new(SdkBody::from_body_1_x(StreamBody::new(
            SyncStream(stream).map_ok(Frame::data).map_err(|e| e),
        )));

        let mut req = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(byte_stream);

        if let Some(len) = content_length {
            req = req.content_length(len as i64);
        }

        req.send().await.map_err(|e| {
            error!("Failed to upload stream: {e:?}");
            StorageError::Generic(format!("S3 Upload Error: {e:?}"))
        })?;

        Ok(true)
    }

    #[instrument(skip(self, data), fields(bucket = %self.bucket, key))]
    async fn write_manifest(&self, version: &str, data: Bytes) -> Result<(), StorageError> {
        let path = self.get_manifest_path(version);
        let key = self.key(&path);
        tracing::Span::current().record("key", &key);

        debug!("Uploading manifest...");
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(data))
            .send()
            .await
            .map_err(|e| {
                error!("Failed to upload manifest: {:?}", e);
                StorageError::Generic(format!("S3 Manifest Upload Error: {:?}", e))
            })?;

        Ok(())
    }

    #[instrument(skip(self), fields(bucket = %self.bucket, key))]
    async fn read_file(&self, path: &str) -> Result<Bytes, StorageError> {
        let key = self.key(path);
        tracing::Span::current().record("key", &key);

        debug!("Reading file from S3...");
        let res = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await;

        match res {
            Ok(output) => {
                let data = output.body.collect().await.map_err(|e| {
                    error!("Failed to stream body: {:?}", e);
                    StorageError::Generic(format!("Failed to stream S3 body: {}", e))
                })?;
                Ok(data.into_bytes())
            }
            Err(SdkError::ServiceError(err)) => {
                let inner = err.err();
                if inner.is_no_such_key() {
                    debug!("File not found in S3");
                    Err(StorageError::NotFound(path.to_string()))
                } else {
                    error!("S3 Service Error during read: {:?}", err);
                    Err(StorageError::Generic(format!(
                        "S3 Service Error: {:?}",
                        inner
                    )))
                }
            }
            Err(e) => {
                error!("Unexpected S3 Error: {:?}", e);
                Err(StorageError::Generic(format!("S3 Error: {:?}", e)))
            }
        }
    }

    #[instrument(skip(self), fields(bucket = %self.bucket, key))]
    async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        let key = self.key(path);
        tracing::Span::current().record("key", &key);
        self.exists(&key).await
    }

    #[instrument(skip(self), fields(bucket = %self.bucket, key))]
    async fn get_download_url(&self, path: &str) -> Result<Option<String>, StorageError> {
        let key = self.key(path);
        tracing::Span::current().record("key", &key);

        let Some(duration) = self.presign_duration else {
            return Ok(None);
        };

        let cfg = PresigningConfig::expires_in(duration)
            .map_err(|e| StorageError::Generic(format!("Invalid presign config: {}", e)))?;

        let req = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .presigned(cfg)
            .await
            .map_err(|e| {
                error!("Failed to presign URL: {:?}", e);
                StorageError::Generic(format!("S3 Presign Error: {}", e))
            })?;

        Ok(Some(req.uri().to_string()))
    }

    #[instrument(skip(self), fields(bucket = %self.bucket, key))]
    async fn delete_file(&self, path: &str) -> Result<(), StorageError> {
        let key = self.key(path);
        tracing::Span::current().record("key", &key);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to delete file: {:?}", e);
                StorageError::Generic(format!("S3 Delete Error: {:?}", e))
            })?;
        Ok(())
    }
}
