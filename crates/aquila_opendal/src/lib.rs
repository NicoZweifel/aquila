//! # Aquila OpenDAL Storage
//! [![Crates.io](https://img.shields.io/crates/v/aquila_opendal.svg)](https://crates.io/crates/aquila_opendal)
//! [![Downloads](https://img.shields.io/crates/d/aquila_opendal.svg)](https://crates.io/crates/aquila_opendal)
//! [![Docs](https://docs.rs/aquila_opendal/badge.svg)](https://docs.rs/aquila_opendal/)
//!
//! A storage backend powered by [Apache OpenDAL](https://opendal.apache.org/).
//!
//! Allows the server to be backed by any storage service supported by OpenDAL, including
//! the file system, AWS S3, GCS, Azure Blob Storage and more.
//!
//! ## Usage
//!
//! ```no_run
//! # use aquila_opendal::OpendalStorage;
//! # use opendal::{Operator, services::Gcs};
//! # async fn run() {
//! let mut builder = Gcs::default();
//! builder.bucket("my-gcs-bucket");
//!
//! let op = Operator::new(builder).unwrap().finish();
//! let storage = OpendalStorage::new(op);
//! # }
//! ```

use aquila_core::prelude::*;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use opendal::Operator;
use std::pin::Pin;

#[derive(Clone)]
pub struct OpendalStorage {
    op: Operator,
}

impl OpendalStorage {
    /// Create a new storage from an OpenDAL Operator.
    /// The Operator can be configured for any supported backend e.g., s3, fs, gcs, etc.
    pub fn new(op: Operator) -> Self {
        Self { op }
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        let exists = self
            .op
            .exists(path)
            .await
            .map_err(|e| StorageError::Generic(e.to_string()))?;

        if exists {
            println!("Blob already exists in opendal storage!");
        }

        Ok(exists)
    }
}

impl StorageBackend for OpendalStorage {
    async fn write_blob(&self, hash: &str, data: Bytes) -> Result<bool, StorageError> {
        let path = hash.to_string();
        let data = data.clone();

        if self.exists(&path).await? {
            return Ok(false);
        }

        self.op
            .write(&path, data)
            .await
            .map_err(|e| StorageError::Generic(format!("OpenDAL Write Error: {}", e)))?;

        Ok(true)
    }

    async fn write_stream(
        &self,
        hash: &str,
        mut stream: Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>,
        _content_length: Option<u64>,
    ) -> Result<bool, StorageError> {
        let path = hash.to_string();
        if self.exists(&path).await? {
            return Ok(false);
        }

        let mut writer = self
            .op
            .writer(&path)
            .await
            .map_err(|e| StorageError::Generic(format!("OpenDAL init error: {e}")))?;

        while let Some(res) = stream.next().await {
            let chunk = res.map_err(StorageError::Io)?;
            writer
                .write(chunk)
                .await
                .map_err(|e| StorageError::Generic(format!("OpenDAL write error: {e}")))?;
        }

        writer
            .close()
            .await
            .map_err(|e| StorageError::Generic(format!("OpenDAL close error: {e}")))?;

        Ok(true)
    }

    async fn write_manifest(&self, version: &str, data: Bytes) -> Result<(), StorageError> {
        let path = self.get_manifest_path(version);
        let data = data.clone();

        self.op
            .write(&path, data)
            .await
            .map_err(|e| StorageError::Generic(format!("OpenDAL Manifest Error: {e}")))?;

        Ok(())
    }

    async fn read_file(&self, path: &str) -> Result<Bytes, StorageError> {
        let path = path.to_string();

        match self.op.read(&path).await {
            Ok(buffer) => Ok(buffer.to_bytes()),
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Err(StorageError::NotFound(path)),
            Err(e) => Err(StorageError::Generic(e.to_string())),
        }
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        self.exists(path).await
    }

    async fn delete_file(&self, path: &str) -> Result<(), StorageError> {
        let path = path.to_string();

        self.op
            .delete(&path)
            .await
            .map_err(|e| StorageError::Generic(e.to_string()))
    }
}
