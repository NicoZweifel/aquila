use super::error::ApiError;
use crate::prelude::*;

use aquila_core::prelude::*;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};

use bytes::Bytes;

/// GET /manifest/{version}
pub async fn get<S: AquilaServices>(
    State(state): State<AppState<S>>,
    _: ScopedUser<ManifestRead>,
    Path(version): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let storage = state.storage();
    let path = storage.get_manifest_path(version.as_str());
    let data = storage.read_file(&path).await?;

    // Validate
    let _manifest: AssetManifest = serde_json::from_slice(&data)?;

    let res = Json(serde_json::from_slice::<serde_json::Value>(&data)?);

    Ok(res)
}

#[derive(serde::Deserialize)]
pub struct PublishParams {
    #[serde(default = "default_true")]
    latest: bool,
}

fn default_true() -> bool {
    true
}

/// POST /manifest
pub async fn publish<S: AquilaServices>(
    State(state): State<AppState<S>>,
    _: ScopedUser<ManifestPublish>,
    Query(params): Query<PublishParams>,
    Json(manifest): Json<AssetManifest>,
) -> Result<impl IntoResponse, ApiError> {
    let data = Bytes::from(serde_json::to_vec_pretty(&manifest)?);
    let storage = state.storage();

    storage
        .write_manifest(&manifest.version, data.clone())
        .await?;

    if params.latest {
        storage.write_manifest("latest", data).await?;
    }

    Ok(StatusCode::CREATED)
}
