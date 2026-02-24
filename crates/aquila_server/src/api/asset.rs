use super::error::ApiError;
use crate::prelude::*;

use aquila_core::prelude::*;

use axum::{
    extract::{Path, Request, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};

use bytes::Bytes;
use futures::TryStreamExt;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tracing::error;

/// GET /assets/{hash}
pub async fn download<S: AquilaServices>(
    State(state): State<AppState<S>>,
    _: ScopedUser<AssetDownload>,
    Path(hash): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let storage = state.storage();
    let data = storage.read_file(&hash).await?;
    if let Some(url) = storage.get_download_url(&hash).await? {
        return Ok(Redirect::temporary(&url).into_response());
    }

    let res = storage
        .get_download_url(&hash)
        .await?
        .map(|url| Redirect::temporary(&url).into_response())
        .unwrap_or_else(||
            // TODO set Content-Type based on manifest info
            data.into_response());

    Ok(res)
}

/// POST /assets
/// Accepts raw body, calculates SHA256, stores it. Returns the Hash.
pub async fn upload_asset<S: AquilaServices>(
    State(state): State<AppState<S>>,
    _: ScopedUser<AssetUpload>,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let mut hasher = Sha256::new();
    hasher.update(&body);
    let hash = hex::encode(hasher.finalize());

    let status = if state.storage().write_blob(&hash, body).await? {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };

    Ok((status, hash))
}

/// PUT /assets/stream/{hash}
pub async fn upload<S: AquilaServices>(
    State(state): State<AppState<S>>,
    _: ScopedUser<AssetUpload>,
    Path(hash): Path<String>,
    request: Request,
) -> Result<impl IntoResponse, ApiError> {
    let content_length = request
        .headers()
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|val| val.to_str().ok())
        .and_then(|val| val.parse::<u64>().ok());

    let hasher = Arc::new(Mutex::new(Sha256::new()));
    let hasher_writer = hasher.clone();
    let stream = request
        .into_body()
        .into_data_stream()
        .map_err(std::io::Error::other)
        .map_ok(move |chunk| {
            if let Ok(mut h) = hasher_writer.lock() {
                h.update(&chunk);
            }
            chunk
        });

    let pinned_stream = Box::pin(stream);
    let storage = state.storage();
    let created = storage
        .write_stream(&hash, pinned_stream, content_length)
        .await?;

    if created {
        let calculated_hash = {
            let hasher_guard = hasher.lock().map_err(|_| {
                ApiError::from(anyhow::anyhow!("Internal Error: Hasher mutex poisoned"))
            })?;
            hex::encode(hasher_guard.clone().finalize())
        };

        if calculated_hash != hash {
            error!(
                "Hash mismatch for upload {hash}. Calculated: {calculated_hash}. Deleting file."
            );

            if let Err(e) = storage.delete_file(&hash).await {
                error!("Failed to delete corrupted file {hash}: {e}");
            }

            return Err(ApiError::from(StorageError::System(format!(
                "Integrity check failed. Expected {hash}, got {calculated_hash}"
            ))));
        };
    }

    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };

    Ok((status, hash))
}
