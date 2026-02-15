use std::{collections::HashMap, fmt, time::Duration};

use serde::{Deserialize, Serialize};

use tauri::{AppHandle, Emitter, async_runtime};

use tokio::{sync::mpsc, time::timeout};

use uuid::Uuid;

use crate::background::ClientRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum GrpcError {
  Internal { message: String },
}

impl fmt::Display for GrpcError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      GrpcError::Internal { message } => write!(f, "Internal error: {}", message),
    }
  }
}

impl From<tauri::Error> for GrpcError {
  fn from(e: tauri::Error) -> Self {
    GrpcError::Internal {
      message: e.to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetadataValue {
  Text(String),
  Binary(Vec<u8>),
}

pub type Metadata = HashMap<String, Vec<MetadataValue>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Frame {
  Header { header: Metadata },
  Data { data: Vec<u8> },
  Trailer { trailer: Metadata },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum GrpcEvent {
  Success { request_id: Uuid, data: Frame },
  Error { request_id: Uuid, error: GrpcError },
}

#[tauri::command]
pub fn grpc(
  state: tauri::State<'_, mpsc::UnboundedSender<ClientRequest>>,
  app: AppHandle,
  path: String,
  metadata: Metadata,
  body: Vec<u8>,
) -> Result<Uuid, GrpcError> {
  let request_id = Uuid::now_v7();
  log::debug!(
    "grpc command invoked: request_id={} path={} metadata_len={} body_len={}",
    request_id,
    path,
    metadata.len(),
    body.len()
  );

  let (response_sender, mut response_receiver) = mpsc::channel(1);

  if let Err(e) = state.send(ClientRequest {
    request_id,
    path,
    metadata,
    body,
    sender: response_sender,
  }) {
    let error_msg = format!("failed to send request to server: {}", e);
    log::error!("request {}: {}", request_id, error_msg);

    match app.emit(
      "grpc-response",
      GrpcEvent::Error {
        request_id,
        error: GrpcError::Internal {
          message: error_msg.clone(),
        },
      },
    ) {
      Ok(_) => log::debug!("request {}: emitted initial error event", request_id),
      Err(em) => log::debug!(
        "request {}: failed to emit initial error event: {}",
        request_id,
        em
      ),
    }

    return Err(GrpcError::Internal { message: error_msg });
  }

  log::debug!(
    "request {}: sent to background, waiting for response (60s)",
    request_id
  );
  drop(async_runtime::spawn(async move {
    log::debug!("request {}: response waiter started", request_id);

    loop {
      match timeout(Duration::from_secs(60), response_receiver.recv()).await {
        Ok(Some(Ok(data))) => {
          log::debug!("request {}: received data frame", request_id);

          let is_trailer = matches!(&data, Frame::Trailer { .. });

          if let Err(e) = app.emit(
            "grpc-response",
            GrpcEvent::Success {
              request_id,
              data: data.clone(),
            },
          ) {
            log::debug!(
              "request {}: failed to emit success event: {}",
              request_id,
              e
            );
          } else {
            match &data {
              Frame::Data { data: d } => log::debug!(
                "request {}: emitting data frame ({} bytes) first_bytes={:?}",
                request_id,
                d.len(),
                &d[..std::cmp::min(8, d.len())]
              ),
              Frame::Header { header: _ } => {
                log::debug!("request {}: emitting header frame", request_id)
              }
              Frame::Trailer { trailer: _ } => {
                log::debug!("request {}: emitting trailer frame", request_id)
              }
            }
          }

          if is_trailer {
            log::debug!(
              "request {}: trailer frame received, ending response waiter",
              request_id
            );
            break;
          }
        }
        Ok(Some(Err(e))) => {
          log::debug!(
            "request {}: received error from background: {:?}",
            request_id,
            e
          );
          match app.emit(
            "grpc-response",
            GrpcEvent::Error {
              request_id,
              error: e,
            },
          ) {
            Ok(_) => log::debug!(
              "request {}: emitted error event (background err)",
              request_id
            ),
            Err(em) => log::debug!(
              "request {}: failed to emit error event (background err): {}",
              request_id,
              em
            ),
          }
          break;
        }
        Ok(None) => {
          log::debug!("request {}: background response channel closed", request_id);
          match app.emit(
            "grpc-response",
            GrpcEvent::Error {
              request_id,
              error: GrpcError::Internal {
                message: "server response cancelled".to_string(),
              },
            },
          ) {
            Ok(_) => log::debug!(
              "request {}: emitted error event (channel closed)",
              request_id
            ),
            Err(em) => log::debug!(
              "request {}: failed to emit error event (channel closed): {}",
              request_id,
              em
            ),
          }
          break;
        }
        Err(_) => {
          log::debug!("request {}: response timed out", request_id);
          match app.emit(
            "grpc-response",
            GrpcEvent::Error {
              request_id,
              error: GrpcError::Internal {
                message: "server response timed out".to_string(),
              },
            },
          ) {
            Ok(_) => log::debug!("request {}: emitted error event (timeout)", request_id),
            Err(em) => log::debug!(
              "request {}: failed to emit error event (timeout): {}",
              request_id,
              em
            ),
          }
          break;
        }
      }
    }
  }));

  Ok(request_id)
}
