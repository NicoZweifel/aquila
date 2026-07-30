use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    /// Low-level I/O error.
    /// Maps to **HTTP 500 Internal Server Error**.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error.
    /// Maps to **HTTP 500 Internal Server Error**.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// The requested resource (file, path, manifest) was not found.
    /// Maps to **HTTP 404 Not Found**.
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// The user request was invalid (e.g., bad path format).
    /// Maps to **HTTP 400 Bad Request**.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Generic system or backend-specific failure (e.g., S3 SDK error).
    /// Maps to **HTTP 500 Internal Server Error**.
    #[error("Storage system failure: {0}")]
    System(String),

    /// The requested feature is not supported by the configured provider.
    /// Maps to **HTTP 501 Not Implemented**.
    #[error("Feature not supported: {0}")]
    Unsupported(String),
}

#[derive(Error, Debug)]
pub enum AuthError {
    /// The token is invalid.
    /// Maps to **HTTP 401 Unauthorized**.
    #[error("Unauthorized: Credentials invalid")]
    Invalid,

    /// The token is valid but has expired.
    /// Maps to **HTTP 401**.
    #[error("Unauthorized: Credentials expired")]
    Expired,

    /// The token is missing.
    /// Maps to **HTTP 401**.
    #[error("Unauthorized: Credentials missing")]
    Missing,

    /// The user is authenticated but lacks the required scope/permission.
    /// Maps to **HTTP 403 Forbidden**.
    #[error("Insufficient permissions: {0}")]
    Forbidden(String),

    /// Generic system or provider failure (e.g., GitHub API down).
    /// Maps to **HTTP 500 Internal Server Error**.
    #[error("Auth system failure: {0}")]
    System(String),

    /// The requested feature is not supported by the configured provider.
    /// Maps to **HTTP 501 Not Implemented**.
    #[error("Feature not supported: {0}")]
    Unsupported(String),
}

#[derive(Debug, Error)]
pub enum ComputeError {
    /// The user provided invalid arguments (e.g., invalid image, bad resource limits).
    /// Maps to **HTTP 400 Bad Request**.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// The requested job or resource was not found.
    /// Maps to **HTTP 404 Not Found**.
    #[error("Job {0} not found")]
    NotFound(String),

    /// Internal infrastructure or provider failure (e.g., AWS Batch error).
    /// Maps to **HTTP 500 Internal Server Error**.
    #[error("Compute system failure: {0}")]
    System(String),

    /// The backend does not support this feature.
    /// Maps to **HTTP 501 Not Implemented**.
    #[error("Feature not supported: {0}")]
    Unsupported(String),
}
