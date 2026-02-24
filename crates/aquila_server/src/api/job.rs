use aquila_core::prelude::*;

use axum::{
    Json,
    extract::ws::{Message, WebSocket},
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
};

use futures::StreamExt;
use tracing::error;

use crate::api::error::ApiError;
use crate::prelude::*;

/// POST /jobs/run
pub async fn run<S: AquilaServices>(
    State(state): State<AppState<S>>,
    _: ScopedUser<JobRun>,
    Json(task): Json<JobRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let res = state.compute().run(task).await.map(Json)?;
    Ok(res)
}

/// POST /jobs/:id/stop
pub async fn stop<S: AquilaServices>(
    State(state): State<AppState<S>>,
    _: ScopedUser<JobRun>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    state.compute().stop(&id).await?;
    Ok(StatusCode::OK)
}

/// GET /jobs/:id/logs
pub async fn get_logs<S: AquilaServices>(
    State(state): State<AppState<S>>,
    _: ScopedUser<JobAttach>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let logs = state.compute().get_logs(&id).await?;
    Ok(logs)
}

/// GET /jobs/:id
pub async fn get_status<S: AquilaServices>(
    State(state): State<AppState<S>>,
    _: ScopedUser<JobAttach>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let status = state.compute().get_status(&id).await?;
    Ok(Json(status))
}

/// GET /jobs/:id/attach
pub async fn attach<S: AquilaServices>(
    State(state): State<AppState<S>>,
    _: ScopedUser<JobAttach>,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, ApiError> {
    let res = ws.on_upgrade(move |socket| handle_attach_socket(state, id, socket));
    Ok(res)
}

async fn handle_attach_socket<S: AquilaServices>(
    state: AppState<S>,
    id: String,
    mut socket: WebSocket,
) {
    let mut compute_stream = match state.compute().attach(&id).await {
        Ok(s) => s,
        Err(e) => {
            let _ = socket
                .send(Message::Text(format!("Error: {:?}", e).into()))
                .await;
            return;
        }
    };

    tracing::info!("Attached to job {}", id);

    loop {
        tokio::select! {
            maybe_log = compute_stream.next() => {
                match maybe_log {
                    Some(Ok(log_output)) => {
                        match serde_json::to_vec(&log_output) {
                            Ok(bytes) => {
                                if socket.send(Message::Binary(bytes.into())).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => error!("Serialization error: {:?}", e),
                        }
                    },
                    Some(Err(e)) => {
                        let _ = socket.send(Message::Text(format!("Stream Error: {:?}", e).into())).await;
                    },
                    None => {
                        break;
                    }
                }
            }
            client_msg = socket.recv() => {
                match client_msg {
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Ping(_))) => {
                    }
                    Some(Err(_)) | None => break,
                    _ => {}
                }
            }
        }
    }

    let _ = socket.send(Message::Close(None)).await;
    tracing::info!("Detached from job {}", id);
}
