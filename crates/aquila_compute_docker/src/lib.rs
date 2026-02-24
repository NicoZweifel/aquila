use aquila_core::prelude::*;

use bollard::{
    Docker,
    config::{ContainerCreateBody, ContainerStateStatusEnum},
    container::LogOutput as DockerLogOutput,
    models::{DeviceRequest, HostConfig},
    query_parameters::StartContainerOptions,
    query_parameters::{
        CreateContainerOptions, DownloadFromContainerOptions, LogsOptions, StopContainerOptions,
    },
};

use futures::{StreamExt, stream::BoxStream};
use std::collections::HashMap;

const OUTPUT_ENV_VAR: &str = "AQUILA_OUTPUT";
const OUTPUT_PATH: &str = "/tmp/aquila_output.json";

#[derive(Clone)]
pub struct DockerComputeBackend {
    client: Docker,
}

impl DockerComputeBackend {
    /// Connects to the local Docker socket (defaults to /var/run/docker.sock on Linux).
    pub async fn connect_local() -> Result<Self, ComputeError> {
        let client = Docker::connect_with_local_defaults()
            .map_err(|e| ComputeError::System(format!("Failed to connect to Docker: {}", e)))?;
        Ok(Self { client })
    }

    /// Reads /aquila/output.json from the container.
    async fn fetch_outputs(&self, id: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();

        let mut stream = self.client.download_from_container(
            id,
            Some(DownloadFromContainerOptions {
                path: "/aquila/output.json".to_string(),
            }),
        );

        let mut tar_buffer = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(bytes) => tar_buffer.extend_from_slice(&bytes),
                Err(_) => return map,
            }
        }

        if tar_buffer.is_empty() {
            return map;
        }

        let mut archive = tar::Archive::new(&tar_buffer[..]);

        if let Ok(entries) = archive.entries() {
            for entry in entries {
                if let Ok(mut file) = entry {
                    let is_output_file = file
                        .path()
                        .map(|p| p.to_string_lossy().ends_with("output.json"))
                        .unwrap_or(false);

                    if is_output_file {
                        let mut json_str = String::new();
                        if std::io::Read::read_to_string(&mut file, &mut json_str).is_ok() {
                            if let Ok(parsed) = serde_json::from_str(&json_str) {
                                map = parsed;
                            }
                        }
                        break;
                    }
                }
            }
        }

        map
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
            .start_container(&name, None::<StartContainerOptions>)
            .await
            .map_err(|e| ComputeError::InvalidRequest(format!("Failed to start: {}", e)))?;

        Ok(JobResult {
            id: name,
            status: JobStatus::running(),
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

    async fn stop(&self, id: &str) -> Result<(), ComputeError> {
        self.client
            .stop_container(id, None::<StopContainerOptions>)
            .await
            .map_err(|e| ComputeError::System(format!("Failed to stop container: {}", e)))
    }

    async fn get_logs(&self, id: &str) -> Result<String, ComputeError> {
        let options = LogsOptions {
            stdout: true,
            stderr: true,
            timestamps: false,
            tail: "all".to_string(),
            ..Default::default()
        };

        let logs = self
            .client
            .logs(id, Some(options))
            .map(|res| match res {
                Ok(DockerLogOutput::StdOut { message }) => {
                    String::from_utf8_lossy(&message).to_string()
                }
                Ok(DockerLogOutput::StdErr { message }) => {
                    String::from_utf8_lossy(&message).to_string()
                }
                Ok(DockerLogOutput::Console { message }) => {
                    String::from_utf8_lossy(&message).to_string()
                }
                Ok(_) => String::new(),
                Err(_) => String::new(),
            })
            .collect::<Vec<String>>()
            .await;

        Ok(logs.join(""))
    }

    async fn get_status(&self, id: &str) -> Result<JobStatus, ComputeError> {
        let res = self
            .client
            .inspect_container(id, None)
            .await
            .map_err(|_| ComputeError::NotFound(format!("Job {} not found", id)))?;

        let container_state = res.state.unwrap_or_default();
        let status = container_state
            .status
            .unwrap_or(ContainerStateStatusEnum::CREATED);

        let exit_code = container_state.exit_code.map(|c| c as i32);
        let finished_at = container_state.finished_at;

        let state = match status {
            ContainerStateStatusEnum::RUNNING => JobState::Running,
            ContainerStateStatusEnum::EXITED => match exit_code {
                Some(0) => JobState::Succeeded,
                _ => JobState::Failed,
            },
            ContainerStateStatusEnum::DEAD => JobState::Failed,
            ContainerStateStatusEnum::PAUSED => JobState::Pending,
            ContainerStateStatusEnum::RESTARTING => JobState::Running,
            _ => JobState::Pending,
        };

        let mut outputs = HashMap::new();
        if state == JobState::Succeeded {
            outputs = self.fetch_outputs(id).await;
        }

        Ok(JobStatus {
            state,
            message: container_state.error,
            exit_code,
            outputs,
            timestamp: finished_at,
        })
    }
}
