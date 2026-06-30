use aquila_core::prelude::*;

use bollard::{
    Docker,
    config::{ContainerCreateBody, ContainerStateStatusEnum},
    container::LogOutput as DockerLogOutput,
    models::{DeviceRequest, HostConfig},
    query_parameters::StartContainerOptions,
    query_parameters::{
        CreateContainerOptions, DownloadFromContainerOptions, ListContainersOptions, LogsOptions,
        StopContainerOptions,
    },
};

use futures::{StreamExt, stream::BoxStream};
use std::collections::HashMap;

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

        let mut labels = req.tags.clone();
        labels.insert("aquila.job".to_string(), "true".to_string());

        let body = ContainerCreateBody {
            image: req.img,
            cmd: Some(req.cmd),
            env: Some(env),
            labels: Some(labels),
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
            ..Default::default()
        };

        let stream = self
            .client
            .logs(job_id, Some(options))
            .map(|chunk_res| match chunk_res {
                Ok(chunk) => {
                    let source = match chunk {
                        DockerLogOutput::StdOut { .. } => LogSource::Stdout,
                        DockerLogOutput::StdErr { .. } => LogSource::Stderr,
                        DockerLogOutput::Console { .. } => LogSource::Console,
                        DockerLogOutput::StdIn { .. } => LogSource::Console,
                    };
                    let msg = String::from_utf8_lossy(chunk.as_ref()).to_string();
                    let (timestamp, msg) = if let Some((ts, rest)) = msg.split_once(' ') {
                        (Some(ts.to_string()), rest.to_string())
                    } else {
                        (None, msg)
                    };

                    Ok(LogOutput {
                        source,
                        timestamp,
                        message: msg,
                    })
                }
                Err(e) => Err(ComputeError::System(e.to_string())),
            })
            .boxed();

        Ok(stream)
    }

    async fn stop(&self, id: &str) -> Result<(), ComputeError> {
        self.client
            .stop_container(id, None::<StopContainerOptions>)
            .await
            .map_err(|e| ComputeError::InvalidRequest(e.to_string()))
    }

    async fn get_logs(&self, id: &str) -> Result<String, ComputeError> {
        let options = LogsOptions {
            stdout: true,
            stderr: true,
            timestamps: true,
            ..Default::default()
        };

        let mut logs = String::new();
        let mut stream = self.client.logs(id, Some(options));

        while let Some(chunk_res) = stream.next().await {
            if let Ok(chunk) = chunk_res {
                let msg = String::from_utf8_lossy(chunk.as_ref()).to_string();
                let (_, msg) = msg.split_once(' ').unwrap_or(("", &msg));
                logs.push_str(msg);
            }
        }

        Ok(logs)
    }

    async fn get_status(&self, id: &str) -> Result<JobStatus, ComputeError> {
        let inspect = self
            .client
            .inspect_container(id, None)
            .await
            .map_err(|e| ComputeError::NotFound(e.to_string()))?;

        let state = inspect.state.unwrap_or_default();
        let status = state.status.unwrap_or(ContainerStateStatusEnum::EMPTY);

        let job_state = match status {
            ContainerStateStatusEnum::CREATED | ContainerStateStatusEnum::RESTARTING => {
                JobState::Pending
            }
            ContainerStateStatusEnum::RUNNING => JobState::Running,
            ContainerStateStatusEnum::EXITED | ContainerStateStatusEnum::DEAD => {
                if state.exit_code == Some(0) {
                    JobState::Succeeded
                } else {
                    JobState::Failed
                }
            }
            ContainerStateStatusEnum::PAUSED
            | ContainerStateStatusEnum::REMOVING
            | ContainerStateStatusEnum::EMPTY => JobState::Failed,
        };

        let outputs = if job_state == JobState::Succeeded || job_state == JobState::Failed {
            self.fetch_outputs(id).await
        } else {
            HashMap::new()
        };

        Ok(JobStatus {
            state: job_state,
            message: state.error,
            exit_code: state.exit_code.map(|c| c as i32),
            outputs,
            timestamp: state.finished_at,
        })
    }

    async fn list_jobs(&self) -> Result<Vec<JobResult>, ComputeError> {
        let mut filters = HashMap::new();
        filters.insert("label".to_string(), vec!["aquila.job=true".to_string()]);

        let options = ListContainersOptions {
            all: true,
            filters: Some(filters),
            ..Default::default()
        };

        let containers = self
            .client
            .list_containers(Some(options))
            .await
            .map_err(|e| ComputeError::System(e.to_string()))?;

        let results = containers
            .into_iter()
            .map(|container| {
                let id = container.id.unwrap_or_default();

                // Map container status to JobState
                use bollard::models::ContainerSummaryStateEnum;
                let job_state = match container.state {
                    Some(ContainerSummaryStateEnum::CREATED)
                    | Some(ContainerSummaryStateEnum::RESTARTING) => JobState::Pending,
                    Some(ContainerSummaryStateEnum::RUNNING) => JobState::Running,
                    Some(ContainerSummaryStateEnum::EXITED)
                    | Some(ContainerSummaryStateEnum::DEAD) => JobState::Failed,
                    _ => JobState::Failed,
                };

                let outputs = container
                    .labels
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|(k, _)| k != "aquila.job")
                    .collect();

                // If it's a specific name, use that, otherwise use ID
                let name = container
                    .names
                    .and_then(|names| names.into_iter().next())
                    .map(|first| first.trim_start_matches('/').to_string())
                    .unwrap_or(id);

                JobResult {
                    id: name,
                    status: JobStatus {
                        state: job_state,
                        outputs,
                        timestamp: None,
                        ..Default::default()
                    },
                }
            })
            .collect();

        Ok(results)
    }
}
