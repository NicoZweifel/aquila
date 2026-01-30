use crate::prelude::*;

use aquila_core::prelude::{scopes::*, *};
use axum::{
    Json,
    extract::{
        Path, Query, Request, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

use tracing::error;

pub struct ApiError(anyhow::Error);

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let Some(err) = self.0.downcast_ref::<StorageError>() {
            return match err {
                StorageError::NotFound(_) => (StatusCode::NOT_FOUND, err.to_string()),
                StorageError::Unsupported(_) => (StatusCode::NOT_IMPLEMENTED, err.to_string()),
                _ => {
                    error!("Internal Server StorageError: {:?}", self.0);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Storage Error".to_string(),
                    )
                }
            }
            .into_response();
        }

        if let Some(err) = self.0.downcast_ref::<ComputeError>() {
            return match err {
                ComputeError::NotFound(_) => (StatusCode::NOT_FOUND, err.to_string()),
                ComputeError::Unsupported(_) => (StatusCode::NOT_IMPLEMENTED, err.to_string()),
                ComputeError::InvalidRequest(_) => {
                    (StatusCode::BAD_REQUEST, format!("Invalid Request: {}", err))
                }
                ComputeError::System(_) => {
                    error!("Internal Server ComputeError: {:?}", self.0);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Compute Error".to_string(),
                    )
                }
            }
            .into_response();
        }

        if let Some(err) = self.0.downcast_ref::<AuthError>() {
            return match err {
                AuthError::Invalid | AuthError::Expired | AuthError::Missing => {
                    (StatusCode::UNAUTHORIZED, err.to_string())
                }
                AuthError::Forbidden(_) => (StatusCode::FORBIDDEN, err.to_string()),
                AuthError::Unsupported(_) => (StatusCode::NOT_IMPLEMENTED, err.to_string()),
                AuthError::System(_) => {
                    error!("Internal Auth Provider Error: {:?}", self.0);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Auth Error".to_string())
                }
            }
            .into_response();
        }

        error!("Internal Server Error: {:?}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error".to_string(),
        )
            .into_response()
    }
}

/// GET /assets/{hash}
pub async fn download_asset<S: AquilaServices>(
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
pub async fn upload_asset_stream<S: AquilaServices>(
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

/// GET /manifest/{version}
pub async fn get_manifest<S: AquilaServices>(
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
pub async fn publish_manifest<S: AquilaServices>(
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

#[derive(serde::Deserialize)]
pub struct AuthCallbackParams {
    code: String,
}

/// GET /auth/login
pub async fn auth_login<S: AquilaServices>(State(state): State<AppState<S>>) -> impl IntoResponse {
    match state.auth().get_login_url() {
        Some(url) => Redirect::temporary(&url).into_response(),
        None => (
            StatusCode::NOT_IMPLEMENTED,
            "Login not supported by this provider",
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
pub struct CreateTokenRequest {
    /// Who is this token for? (e.g., "game_v1", "build_server")
    pub subject: String,
    /// How long should it last?
    ///
    /// Default: 1 year
    pub duration_seconds: Option<u64>,
    /// Optional scopes
    ///
    /// Default: `read`
    pub scopes: Option<Vec<String>>,
}

/// POST /auth/token
pub async fn issue_token<S: AquilaServices>(
    State(state): State<AppState<S>>,
    ScopedUser { user, .. }: ScopedUser<WriteScope>,
    Json(req): Json<CreateTokenRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let scopes = req.scopes.unwrap_or_else(|| vec![READ.to_string()]);
    let is_admin = user.scopes.iter().any(|s| s == ADMIN);
    let privileged_scopes = [ADMIN, WRITE];

    if !is_admin {
        for scope in &scopes {
            if privileged_scopes.contains(&scope.as_str()) {
                return Err(ApiError::from(AuthError::Forbidden(format!(
                    "Insufficient permissions to mint '{scope}' token."
                ))));
            }
        }
    }

    let duration = req.duration_seconds.unwrap_or(31_536_000); // 1 year
    let token = state.jwt().mint(req.subject, scopes, duration)?;

    let res = Json(serde_json::json!({
        "token": token,
        "expires_in": duration
    }));

    Ok(res)
}

/// GET /auth/callback (can be configured, see [`AquilaServerConfig`])
pub async fn auth_callback<S: AquilaServices>(
    State(state): State<AppState<S>>,
    Query(params): Query<AuthCallbackParams>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state
        .auth()
        .exchange_code(&params.code)
        .await
        .map_err(ApiError::from)?;

    let session_token = state.jwt().mint(
        user.id.clone(),
        user.scopes,
        60 * 60 * 24 * 30, // 30 Days
    )?;

    let res = Json(serde_json::json!({
        "status": "success",
        "user": user.id,
        "token": session_token
    }));

    Ok(res)
}

/// Handler: POST /jobs/run
pub async fn run<S: AquilaServices>(
    State(state): State<AppState<S>>,
    _: ScopedUser<JobRun>,
    Json(task): Json<JobRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let res = state.compute().run(task).await.map(Json)?;
    Ok(res)
}

/// Handler: GET /jobs/:id/attach
pub async fn attach<S: AquilaServices>(
    State(state): State<AppState<S>>,
    _: ScopedUser<JobAttach>,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, ApiError> {
    let res = ws.on_upgrade(move |socket| handle_attach_socket(state, id, socket));
    Ok(res)
}

async fn handle_attach_socket<S: AquilaServices>(
    state: AppState<S>,
    id: String,
    mut socket: WebSocket,
) {
    let mut compute_stream = match state.compute().attach(&id).await {
        Ok(s) => s,
        Err(e) => {
            let _ = socket
                .send(Message::Text(format!("Error: {:?}", e).into()))
                .await;
            return;
        }
    };

    tracing::info!("Attached to job {}", id);

    loop {
        tokio::select! {
            maybe_log = compute_stream.next() => {
                match maybe_log {
                    Some(Ok(log_output)) => {
                        match serde_json::to_vec(&log_output) {
                            Ok(bytes) => {
                                if socket.send(Message::Binary(bytes.into())).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => error!("Serialization error: {:?}", e),
                        }
                    },
                    Some(Err(e)) => {
                        let _ = socket.send(Message::Text(format!("Stream Error: {:?}", e).into())).await;
                    },
                    None => {
                        break;
                    }
                }
            }
            client_msg = socket.recv() => {
                match client_msg {
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Ping(_))) => {
                    }
                    Some(Err(_)) | None => break,
                    _ => {}
                }
            }
        }
    }

    let _ = socket.send(Message::Close(None)).await;
    tracing::info!("Detached from job {}", id);
}
