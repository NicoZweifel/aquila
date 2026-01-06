//! # Aquila FileSystem Storage
//! [![Crates.io](https://img.shields.io/crates/v/aquila_fs.svg)](https://crates.io/crates/aquila_fs)
//! [![Downloads](https://img.shields.io/crates/d/aquila_fs.svg)](https://crates.io/crates/aquila_fs)
//! [![Docs](https://docs.rs/aquila_fs/badge.svg)](https://docs.rs/aquila_fs/)
//!
//! A local filesystem backend for Aquila.
//!
//! This crate implements the [`StorageBackend`] trait, storing blobs
//! and manifests directly in the file system.
//!
//! ## Features
//!
//! * **Atomic Writes**: Uses temporary files and rename operations to ensure assets are not read partially or lost during upload.
//!
//! ## Usage
//!
//! ```no_run
//! use aquila_fs::FileSystemStorage;
//!
//! let storage = FileSystemStorage::new("./aquila_data");
//! ```

use aquila_core::prelude::*;
use bytes::Bytes;
use std::path::PathBuf;
use tokio::fs;

async fn atomic_write(path: &std::path::Path, data: Bytes) -> Result<(), StorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.map_err(StorageError::Io)?;
    }

    let tmp_path = path.with_extension("tmp");

    fs::write(&tmp_path, data).await.map_err(StorageError::Io)?;
    fs::rename(&tmp_path, path)
        .await
        .map_err(StorageError::Io)?;

    Ok(())
}

#[derive(Clone)]
pub struct FileSystemStorage {
    root: PathBuf,
}

impl FileSystemStorage {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { root: path.into() }
    }

    fn get_path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }
}

impl StorageBackend for FileSystemStorage {
    async fn write_blob(&self, hash: &str, data: Bytes) -> Result<bool, StorageError> {
        let path = self.get_path(hash);
        if path.exists() {
            return Ok(false);
        }
        atomic_write(&path, data).await?;
        Ok(true)
    }

    async fn write_manifest(&self, version: &str, data: Bytes) -> Result<(), StorageError> {
        let path = self.get_path(&self.get_manifest_path(version));
        atomic_write(&path, data).await?;
        Ok(())
    }

    async fn read_file(&self, path: &str) -> Result<Bytes, StorageError> {
        let path = self.get_path(path);
        match fs::read(&path).await {
            Ok(data) => Ok(Bytes::from(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(StorageError::NotFound(path.to_string_lossy().to_string()))
            }
            Err(e) => Err(StorageError::Io(e)),
        }
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        Ok(self.get_path(path).exists())
    }
}
