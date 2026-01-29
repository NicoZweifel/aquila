use crate::error::*;
use crate::prelude::*;

use bytes::Bytes;
use futures::stream::BoxStream;

/// A trait that acts as a Service Container for injecting dependencies into the server.
///
/// Prevents generics going viral.
pub trait AquilaServices: Clone + Send + Sync + 'static {
    type Storage: StorageBackend;
    type Auth: AuthProvider;
    type Compute: ComputeBackend;
    type Jwt: JwtBackend;

    /// The registered [`StorageBackend`]
    fn storage(&self) -> &Self::Storage;
    /// The registered [`AuthProvider`]
    fn auth(&self) -> &Self::Auth;
    /// The registered [`ComputeBackend`]
    fn compute(&self) -> &Self::Compute;

    /// The registered [`JwtBackend`]
    fn jwt(&self) -> &Self::Jwt;
}

/// A trait for injecting storage logic into the server.
pub trait StorageBackend: Send + Sync + 'static + Clone {
    /// Writes a file blob to the storage backend.
    fn write_blob(
        &self,
        hash: &str,
        data: Bytes,
    ) -> impl Future<Output = Result<bool, StorageError>> + Send;

    /// Writes a file stream to the storage backend.
    fn write_stream(
        &self,
        _hash: &str,
        _stream: BoxStream<'static, Result<Bytes, std::io::Error>>,
        _content_length: Option<u64>,
    ) -> impl Future<Output = Result<bool, StorageError>> + Send {
        async {
            Err(StorageError::Unsupported(
                "Streaming not implemented for this backend".into(),
            ))
        }
    }

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

    /// Optional: Returns a direct download URL (e.g., S3 Presigned URL, CDN URL).
    ///
    /// - If this returns `Ok(Some(url))`, the server will issue a 307 Redirect to that URL.
    /// - If `Ok(None)` (default), the server will download and proxy the file.
    fn get_download_url(
        &self,
        _path: &str,
    ) -> impl Future<Output = Result<Option<String>, StorageError>> + Send {
        async { Ok(None) }
    }

    /// Deletes a file from the storage backend.
    fn delete_file(&self, path: &str) -> impl Future<Output = Result<(), StorageError>> + Send;
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
            Err(AuthError::Unsupported(
                "Login flow not supported by this provider".into(),
            ))
        }
    }
}

/// A trait for injecting compute logic into the server, e.g., running a build/bake pipeline.
pub trait ComputeBackend: Send + Sync + 'static + Clone {
    /// Initialize the backend, e.g., verify AWS credentials or Docker socket.
    fn init(&self) -> impl Future<Output = Result<(), ComputeError>> + Send;

    /// Runs a job and returns a [`JobResult`].
    fn run(&self, req: JobRequest) -> impl Future<Output = Result<JobResult, ComputeError>> + Send;

    /// Attaches to a running job to read a stream of logs.
    fn attach(
        &self,
        id: &str,
    ) -> impl Future<
        Output = Result<BoxStream<'static, Result<LogOutput, ComputeError>>, ComputeError>,
    > + Send;
}

#[derive(Clone)]
pub struct NoComputeBackend;

impl ComputeBackend for NoComputeBackend {
    async fn init(&self) -> Result<(), ComputeError> {
        Err(ComputeError::Unsupported("Not supported!".to_string()))
    }

    async fn run(&self, _req: JobRequest) -> Result<JobResult, ComputeError> {
        Err(ComputeError::Unsupported("Not supported!".to_string()))
    }
    async fn attach(
        &self,
        _id: &str,
    ) -> Result<BoxStream<'static, Result<LogOutput, ComputeError>>, ComputeError> {
        Err(ComputeError::Unsupported("Not supported!".to_string()))
    }
}

pub trait JwtBackend: Clone + Send + Sync + 'static {
    fn mint(
        &self,
        subject: String,
        scopes: Vec<String>,
        duration_seconds: u64,
    ) -> Result<String, AuthError>;

    fn verify(&self, token: &str) -> Result<User, AuthError>;
}

#[derive(Clone)]
pub struct NoJwtBackend;

impl JwtBackend for NoJwtBackend {
    fn mint(
        &self,
        _subject: String,
        _scopes: Vec<String>,
        _duration_seconds: u64,
    ) -> Result<String, AuthError> {
        Err(AuthError::Unsupported("Not supported!".into()))
    }

    fn verify(&self, _token: &str) -> Result<User, AuthError> {
        Err(AuthError::Unsupported("Not supported!".into()))
    }
}
