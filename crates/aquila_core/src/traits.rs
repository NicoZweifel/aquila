use crate::error::*;

use bytes::Bytes;

/// A trait for injecting storage logic into the server.
pub trait StorageBackend: Send + Sync + 'static + Clone {
    /// Writes a file to the storage backend.
    fn write_blob(
        &self,
        hash: &str,
        data: Bytes,
    ) -> impl Future<Output = Result<bool, StorageError>> + Send;

    /// Writes a manifest with the specified version to the storage backend.
    fn write_manifest(
        &self,
        version: &str,
        data: Bytes,
    ) -> impl Future<Output = Result<(), StorageError>> + Send;

    /// Reads a file from the storage backend.
    fn read_file(&self, path: &str) -> impl Future<Output = Result<Bytes, StorageError>> + Send;

    /// Checks if a file exists in the storage backend.
    fn exists(&self, path: &str) -> impl Future<Output = Result<bool, StorageError>> + Send;

    /// Returns a Manifest path for a given version
    fn get_manifest_path(&self, version: &str) -> String {
        format!("manifests/{version}")
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub scopes: Vec<String>,
}

/// A trait for injecting authentication logic into the server.
pub trait AuthProvider: Send + Sync + 'static + Clone {
    /// Verifies a token and returns a User identity if successful.
    fn verify(&self, token: &str) -> impl Future<Output = Result<User, AuthError>> + Send;

    /// Optional: Returns a login url to start an auth flow.
    fn get_login_url(&self) -> Option<String> {
        None
    }

    /// Optional: Exchanges an authorization code for a User identity.
    fn exchange_code(&self, _code: &str) -> impl Future<Output = Result<User, AuthError>> + Send {
        async {
            Err(AuthError::Generic(
                "Login flow not supported by this provider".into(),
            ))
        }
    }
}
