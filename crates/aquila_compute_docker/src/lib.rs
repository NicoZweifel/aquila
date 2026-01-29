use aquila_core::prelude::*;
use bollard::config::ContainerCreateBody;
use bollard::container::LogOutput as DockerLogOutput;
use bollard::query_parameters::{CreateContainerOptions, LogsOptions};
use bollard::{
    Docker,
    models::{DeviceRequest, HostConfig},
};
use futures::{StreamExt, stream::BoxStream};

#[derive(Clone)]
pub struct DockerComputeBackend {
    client: Docker,
}

impl DockerComputeBackend {
    /// Connects to the local Docker socket (defaults to /var/run/docker.sock on Linux)
    pub async fn connect_local() -> Result<Self, ComputeError> {
        let client = Docker::connect_with_local_defaults()
            .map_err(|e| ComputeError::System(format!("Failed to connect to Docker: {}", e)))?;
        Ok(Self { client })
    }
}

impl ComputeBackend for DockerComputeBackend {
    async fn init(&self) -> Result<(), ComputeError> {
        self.client
            .version()
            .await
            .map_err(|e| ComputeError::System(format!("Docker unavailable: {}", e)))?;

        Ok(())
    }

    async fn run(&self, req: JobRequest) -> Result<JobResult, ComputeError> {
        let job_id = uuid::Uuid::new_v4().to_string();
        let name = format!("aquila-job-{}", job_id);

        let device_requests = req.gpu.map(|driver| {
            vec![DeviceRequest {
                driver: Some(driver),
                count: Some(-1),
                capabilities: Some(vec![vec!["gpu".to_string()]]),
                ..Default::default()
            }]
        });

        let env: Vec<String> = req
            .env
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let options = CreateContainerOptions {
            name: name.clone().into(),
            ..Default::default()
        };

        let body = ContainerCreateBody {
            image: req.img,
            cmd: Some(req.cmd),
            env: Some(env),
            host_config: Some(HostConfig {
                device_requests,
                auto_remove: Some(req.remove),
                ..Default::default()
            }),
            ..Default::default()
        };

        self.client
            .create_container(Some(options), body)
            .await
            .map_err(|e| ComputeError::InvalidRequest(e.to_string()))?;

        self.client
            .start_container(name.as_str(), None)
            .await
            .map_err(|e| ComputeError::InvalidRequest(format!("Failed to start: {}", e)))?;

        Ok(JobResult {
            id: name,
            status: JobStatus::Running,
        })
    }

    async fn attach(
        &self,
        job_id: &str,
    ) -> Result<BoxStream<'static, Result<LogOutput, ComputeError>>, ComputeError> {
        let options = LogsOptions {
            follow: true,
            stdout: true,
            stderr: true,
            timestamps: true,
            tail: "all".to_string(),
            ..Default::default()
        };

        let stream = self.client.logs(job_id, Some(options));
        let mapped_stream = stream.map(|res| match res {
            Ok(output) => {
                let (source, bytes) = match output {
                    DockerLogOutput::StdOut { message } => (LogSource::Stdout, message),
                    DockerLogOutput::StdErr { message } => (LogSource::Stderr, message),
                    DockerLogOutput::Console { message } => (LogSource::Console, message),
                    DockerLogOutput::StdIn { message } => (LogSource::Console, message),
                };

                let full_line = String::from_utf8_lossy(&bytes);
                let (ts, msg) = match full_line.split_once(' ') {
                    Some((t, m)) => (Some(t.to_string()), m.to_string()),
                    None => (None, full_line.to_string()),
                };

                Ok(LogOutput {
                    source,
                    timestamp: ts,
                    message: msg,
                })
            }
            Err(e) => Err(ComputeError::System(e.to_string())),
        });

        Ok(mapped_stream.boxed())
    }
}
