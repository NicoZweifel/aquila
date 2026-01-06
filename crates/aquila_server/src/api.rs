use crate::auth::AuthenticatedUser;
use crate::state::AppState;
use aquila_core::prelude::*;
use axum::extract::Query;
use axum::response::Redirect;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use sha2::{Digest, Sha256};

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
        self.0
            .downcast_ref::<StorageError>()
            .map(|storage_err| match storage_err {
                StorageError::NotFound(_) => (StatusCode::NOT_FOUND, "Asset not found".to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, storage_err.to_string()),
            })
            .unwrap_or_else(|| {
                self.0
                    .downcast_ref::<AuthError>()
                    .map(|_| (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))
                    .unwrap_or((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Internal Server Error: {}", self.0),
                    ))
            })
            .into_response()
    }
}

fn check_scope(user: &AuthenticatedUser, required: &str) -> Result<(), ApiError> {
    if user.0.scopes.iter().any(|s| s == "admin" || s == required) {
        Ok(())
    } else {
        Err(ApiError::from(AuthError::Forbidden(format!(
            "Missing permission: '{}' scope required.",
            required
        ))))
    }
}

/// GET /assets/{hash}
pub async fn download_asset<S: StorageBackend, A: AuthProvider>(
    State(state): State<AppState<S, A>>,
    auth_user: AuthenticatedUser,
    Path(hash): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    check_scope(&auth_user, "read")?;
    let data = state.storage.read_file(&hash).await?;
    // TODO set Content-Type based on manifest info
    Ok(data)
}

/// POST /assets
/// Accepts raw body, calculates SHA256, stores it. Returns the Hash.
pub async fn upload_asset<S: StorageBackend, A: AuthProvider>(
    State(state): State<AppState<S, A>>,
    auth_user: AuthenticatedUser,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    check_scope(&auth_user, "write")?;

    let mut hasher = Sha256::new();
    hasher.update(&body);
    let hash = hex::encode(hasher.finalize());

    let status = if state.storage.write_blob(&hash, body).await? {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };

    Ok((status, hash))
}

/// GET /manifest/{version}
pub async fn get_manifest<S: StorageBackend, A: AuthProvider>(
    State(state): State<AppState<S, A>>,
    auth_user: AuthenticatedUser,
    Path(version): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    check_scope(&auth_user, "read")?;

    let path = state.storage.get_manifest_path(version.as_str());
    let data = state.storage.read_file(&path).await?;

    // Validate
    let _manifest: AssetManifest = serde_json::from_slice(&data)?;

    Ok(Json(serde_json::from_slice::<serde_json::Value>(&data)?))
}

/// POST /manifest
pub async fn publish_manifest<S: StorageBackend, A: AuthProvider>(
    State(state): State<AppState<S, A>>,
    auth_user: AuthenticatedUser,
    Json(manifest): Json<AssetManifest>,
) -> Result<impl IntoResponse, ApiError> {
    check_scope(&auth_user, "write")?;

    let data = Bytes::from(serde_json::to_vec_pretty(&manifest)?);

    state
        .storage
        .write_manifest(&manifest.version, data)
        .await?;

    Ok(StatusCode::CREATED)
}

#[derive(serde::Deserialize)]
pub struct AuthCallbackParams {
    code: String,
}

/// GET /auth/login
pub async fn auth_login<S: StorageBackend, A: AuthProvider>(
    State(state): State<AppState<S, A>>,
) -> impl IntoResponse {
    match state.auth.get_login_url() {
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

pub async fn issue_token<S: StorageBackend, A: AuthProvider>(
    State(state): State<AppState<S, A>>,
    auth_user: AuthenticatedUser,
    Json(req): Json<CreateTokenRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // TODO maybe limit this to admin or separate scope
    check_scope(&auth_user, "write")?;

    let scopes = req.scopes.unwrap_or_else(|| vec!["read".to_string()]);
    if scopes.contains(&"admin".to_string()) {
        return Err(ApiError::from(AuthError::Forbidden(
            "Cannot mint admin tokens.".into(),
        )));
    }

    let duration = req.duration_seconds.unwrap_or(31_536_000); // 1 year
    let token = state.jwt_service.mint(req.subject, scopes, duration)?;

    Ok(Json(serde_json::json!({
        "token": token,
        "expires_in": duration
    })))
}

pub async fn auth_callback<S: StorageBackend, A: AuthProvider>(
    State(state): State<AppState<S, A>>,
    Query(params): Query<AuthCallbackParams>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state
        .auth
        .exchange_code(&params.code)
        .await
        .map_err(ApiError::from)?;

    let session_token = state.jwt_service.mint(
        user.id.clone(),
        user.scopes,
        60 * 60 * 24 * 30, // 30 Days
    )?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "user": user.id,
        "token": session_token
    })))
}
