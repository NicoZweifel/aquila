use serde::{Deserialize, Serialize};

/// A request to run a new compute job on the server's [`ComputeBackend`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobRequest {
    /// The image to use for the job. If [`None`], the default image will be used.
    ///
    /// [`None`] if the [`ComputeBackend`] doesn't support it, or if using the default image.
    pub img: Option<String>,
    /// The profile to use (e.g., "default", "deploy", "gpu")
    /// Can be used to map to a specific Job Definition on the server.
    pub profile: Option<String>,
    /// The queue to use for the job.
    ///
    /// [`None`] if the [`ComputeBackend`] doesn't support it, or if using the default queue.
    pub queue: Option<String>,
    /// Command to execute.
    pub cmd: Vec<String>,
    /// Environment variables (e.g. --env REPO_URL=... --env FOO=bar)
    pub env: Vec<(String, String)>,
    /// Override vCPU count
    pub cpu: Option<String>,
    /// Override Memory in MiB
    pub memory: Option<String>,
    /// GPU driver to use. If [`None`], no gpu is attached.
    pub gpu: Option<String>,
    /// Remove the job/container after it finishes.
    pub remove: bool,
}

/// The result of a compute job request on the server's [`ComputeBackend`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    /// The ID of the compute job.
    pub id: String,
    /// The status of the compute job.
    pub status: JobStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Pending,
    Running,
    Succeeded,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogSource {
    Stdout,
    Stderr,
    Console,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogOutput {
    pub source: LogSource,
    /// RFC3339 timestamp string ,e.g., "2026-01-01T01:00:00Z".
    pub timestamp: Option<String>,
    pub message: String,
}
