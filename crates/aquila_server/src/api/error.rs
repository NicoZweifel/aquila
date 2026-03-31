use aquila_core::prelude::*;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

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
