use super::error::ApiError;
use crate::prelude::*;

use aquila_core::prelude::{
    scopes::{ADMIN, READ, WRITE},
    *,
};

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};

#[derive(serde::Deserialize)]
pub struct AuthCallbackParams {
    code: String,
}

/// GET /auth/login
pub async fn login<S: AquilaServices>(State(state): State<AppState<S>>) -> impl IntoResponse {
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

    if !is_admin {
        let privileged_scopes = [ADMIN, WRITE];
        let violation = scopes
            .iter()
            .find(|s| privileged_scopes.contains(&s.as_str()));

        if let Some(scope) = violation {
            return Err(ApiError::from(AuthError::Forbidden(format!(
                "Insufficient permissions to mint '{scope}' token."
            ))));
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
pub async fn callback<S: AquilaServices>(
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
