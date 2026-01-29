use aquila_core::prelude::*;

use aws_sdk_batch::error::SdkError;
use aws_sdk_batch::{
    Client as BatchClient,
    types::{
        ContainerOverrides, ContainerProperties, JobDefinitionType, JobStatus as AwsJobStatus,
        KeyValuePair, ResourceRequirement, ResourceType,
    },
};
use aws_sdk_cloudwatchlogs::Client as LogsClient;
use aws_sdk_cloudwatchlogs::operation::get_log_events::GetLogEventsError;

use futures::stream::{self, BoxStream, StreamExt};

use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct AwsBatchBackend {
    batch: BatchClient,
    logs: LogsClient,
    default_queue: String,
    profiles: HashMap<String, String>,
}

impl AwsBatchBackend {
    pub fn new(
        config: &aws_config::SdkConfig,
        default_queue: impl Into<String>,
        profiles: HashMap<String, String>,
    ) -> Self {
        Self {
            batch: BatchClient::new(config),
            logs: LogsClient::new(config),
            default_queue: default_queue.into(),
            profiles,
        }
    }

    /// Resolves the base Job Definition ARN from the request profile.
    fn get_base_arn(&self, profile: Option<&str>) -> Result<String, ComputeError> {
        let key = profile.unwrap_or("default");
        self.profiles
            .get(key)
            .cloned()
            .ok_or_else(|| ComputeError::InvalidRequest(format!("Profile '{}' not found", key)))
    }

    /// Creates a new, dynamic Job Definition based on a template ARN + Request Overrides.
    async fn register_dynamic_definition(
        &self,
        base_arn: &str,
        req: &JobRequest,
    ) -> Result<String, ComputeError> {
        let desc = self
            .batch
            .describe_job_definitions()
            .job_definitions(base_arn)
            .send()
            .await
            .map_err(|e| {
                ComputeError::System(format!("Failed to describe base definition: {:?}", e))
            })?;

        let base_def = desc.job_definitions().first().ok_or_else(|| {
            ComputeError::NotFound(format!("Base definition {} not found", base_arn))
        })?;

        let base_props = base_def.container_properties().ok_or_else(|| {
            ComputeError::System("Base definition missing container properties".into())
        })?;

        let mut requirements = base_props.resource_requirements.clone().unwrap_or_default();
        if let Some(cpu) = &req.cpu {
            requirements.retain(|r| r.r#type != Some(ResourceType::Vcpu));
            requirements.push(
                ResourceRequirement::builder()
                    .r#type(ResourceType::Vcpu)
                    .value(cpu)
                    .build(),
            );
        }

        if let Some(mem) = &req.memory {
            requirements.retain(|r| r.r#type != Some(ResourceType::Memory));
            requirements.push(
                ResourceRequirement::builder()
                    .r#type(ResourceType::Memory)
                    .value(mem)
                    .build(),
            );
        }

        if let Some(_gpu) = &req.gpu {
            requirements.retain(|r| r.r#type != Some(ResourceType::Gpu));
            requirements.push(
                ResourceRequirement::builder()
                    .r#type(ResourceType::Gpu)
                    // 1 GPU for now
                    .value("1")
                    .build(),
            );
        }

        let image = req
            .img
            .clone()
            .or_else(|| base_props.image.clone())
            .ok_or_else(|| {
                ComputeError::InvalidRequest(
                    "No image specified in request or base definition".into(),
                )
            })?;

        let mut props_builder = ContainerProperties::builder()
            .image(image)
            .set_resource_requirements(Some(requirements))
            .set_environment(base_props.environment.clone())
            .set_secrets(base_props.secrets.clone())
            .set_volumes(base_props.volumes.clone())
            .set_mount_points(base_props.mount_points.clone())
            .set_ulimits(base_props.ulimits.clone())
            .set_network_configuration(base_props.network_configuration.clone())
            .set_log_configuration(base_props.log_configuration.clone());

        if let Some(role) = base_props.job_role_arn() {
            props_builder = props_builder.job_role_arn(role);
        }
        if let Some(role) = base_props.execution_role_arn() {
            props_builder = props_builder.execution_role_arn(role);
        }
        if let Some(fargate) = base_props.fargate_platform_configuration() {
            props_builder = props_builder.fargate_platform_configuration(fargate.clone());
        }
        if let Some(linux) = base_props.linux_parameters() {
            props_builder = props_builder.linux_parameters(linux.clone());
        }

        let name = format!("aquila-dynamic-{}", Uuid::new_v4());
        self.batch
            .register_job_definition()
            .job_definition_name(name)
            .r#type(JobDefinitionType::Container)
            .container_properties(props_builder.build())
            .set_retry_strategy(base_def.retry_strategy.clone())
            .set_timeout(base_def.timeout.clone())
            .set_platform_capabilities(base_def.platform_capabilities.clone())
            .send()
            .await
            .map(|r| r.job_definition_arn)
            .map_err(|e| ComputeError::System(format!("Failed to register definition: {:?}", e)))?
            .map(|x| Ok(x))
            .unwrap_or(Err(ComputeError::System(
                "Failed to register definition".to_string(),
            )))
    }
}

impl ComputeBackend for AwsBatchBackend {
    async fn init(&self) -> Result<(), ComputeError> {
        self.batch
            .describe_job_queues()
            .job_queues(&self.default_queue)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| ComputeError::System(format!("AWS Batch error: {}", e)))
    }

    async fn run(&self, req: JobRequest) -> Result<JobResult, ComputeError> {
        let base_arn = self.get_base_arn(req.profile.as_deref())?;
        let job = self.register_dynamic_definition(&base_arn, &req).await?;
        let env: Vec<KeyValuePair> = req
            .env
            .iter()
            .map(|(k, v)| KeyValuePair::builder().name(k).value(v).build())
            .collect();

        let mut builder = ContainerOverrides::builder().set_environment(Some(env));

        if !req.cmd.is_empty() {
            builder = builder.set_command(Some(req.cmd));
        }

        let name = format!("aquila-{}", Uuid::new_v4());
        let queue = req.queue.as_deref().unwrap_or(&self.default_queue);

        self.batch
            .submit_job()
            .job_name(name)
            .job_queue(queue)
            .job_definition(job)
            .container_overrides(builder.build())
            .send()
            .await
            .map(|output| JobResult {
                id: output.job_id.unwrap_or_default(),
                status: JobStatus::Pending,
            })
            .map_err(|e| ComputeError::System(e.to_string()))
    }

    // TODO refactor this into sensible pieces
    async fn attach(
        &self,
        job_id: &str,
    ) -> Result<BoxStream<'static, Result<LogOutput, ComputeError>>, ComputeError> {
        let state = LogStreamState {
            batch: self.batch.clone(),
            logs: self.logs.clone(),
            job_id: job_id.to_string(),
            log_stream_name: None,
            next_token: None,
            buffer: VecDeque::new(),
            finished: false,
            error_count: 0,
        };

        let stream = stream::unfold(state, |mut state| async move {
            loop {
                if state.error_count > 15 {
                    return Some((
                        Err(ComputeError::System("Too many transient errors".into())),
                        state,
                    ));
                }

                if let Some(log) = state.buffer.pop_front() {
                    return Some((Ok(log), state));
                }

                if state.finished {
                    return None;
                }

                if state.log_stream_name.is_none() {
                    match state.batch.describe_jobs().jobs(&state.job_id).send().await {
                        Ok(resp) => {
                            state.error_count = 0;
                            if let Some(job) = resp.jobs().first() {
                                if matches!(
                                    job.status(),
                                    Some(AwsJobStatus::Succeeded | AwsJobStatus::Failed)
                                ) {
                                    state.finished = true;
                                }

                                if let Some(container) = job.container() {
                                    if let Some(ls) = container.log_stream_name() {
                                        state.log_stream_name = Some(ls.to_string());
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            if matches!(e, SdkError::TimeoutError(_) | SdkError::DispatchFailure(_))
                            {
                                state.error_count += 1;
                                tokio::time::sleep(Duration::from_secs(2)).await;
                                continue;
                            } else {
                                return Some((Err(ComputeError::System(e.to_string())), state));
                            }
                        }
                    }

                    if state.log_stream_name.is_none() {
                        if state.finished {
                            return None;
                        }
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                }

                if let Some(ref log_stream) = state.log_stream_name {
                    let mut req = state
                        .logs
                        .get_log_events()
                        .log_group_name("/aws/batch/job")
                        .log_stream_name(log_stream)
                        .start_from_head(true);

                    if let Some(ref token) = state.next_token {
                        req = req.next_token(token);
                    }

                    match req.send().await {
                        Ok(output) => {
                            state.error_count = 0;

                            let events = output.events();
                            if events.is_empty() {
                                if !state.finished {
                                    match state
                                        .batch
                                        .describe_jobs()
                                        .jobs(&state.job_id)
                                        .send()
                                        .await
                                    {
                                        Ok(resp) => {
                                            if let Some(job) = resp.jobs().first() {
                                                if matches!(
                                                    job.status(),
                                                    Some(
                                                        AwsJobStatus::Succeeded
                                                            | AwsJobStatus::Failed
                                                    )
                                                ) {
                                                    state.finished = true;
                                                }
                                            }
                                        }
                                        Err(_) => {}
                                    }
                                }

                                if state.finished {
                                    return None;
                                }

                                tokio::time::sleep(Duration::from_secs(2)).await;
                                continue;
                            }

                            state.next_token = output.next_forward_token.clone();
                            for event in events {
                                let timestamp = event.timestamp().map(|ts| {
                                    use chrono::TimeZone;
                                    chrono::Utc.timestamp_millis_opt(ts).unwrap().to_rfc3339()
                                });

                                state.buffer.push_back(LogOutput {
                                    source: LogSource::Stdout,
                                    timestamp,
                                    message: format!("{}\n", event.message().unwrap_or_default()),
                                });
                            }
                        }
                        Err(e) => {
                            if should_retry(&e) {
                                state.error_count += 1;
                                tokio::time::sleep(Duration::from_secs(2)).await;
                                continue;
                            } else {
                                return Some((Err(ComputeError::System(e.to_string())), state));
                            }
                        }
                    }
                }
            }
        });

        Ok(stream.boxed())
    }
}

fn should_retry(err: &SdkError<GetLogEventsError>) -> bool {
    match err {
        SdkError::DispatchFailure(_) | SdkError::TimeoutError(_) => true,
        SdkError::ServiceError(context) => match context.err() {
            GetLogEventsError::ServiceUnavailableException(_) => true,
            GetLogEventsError::ResourceNotFoundException(_) => true,
            GetLogEventsError::InvalidParameterException(_) => false,
            _ => context.raw().status().is_server_error(),
        },
        _ => false,
    }
}

struct LogStreamState {
    batch: BatchClient,
    logs: LogsClient,
    job_id: String,
    log_stream_name: Option<String>,
    next_token: Option<String>,
    buffer: VecDeque<LogOutput>,
    finished: bool,
    error_count: u32,
}
