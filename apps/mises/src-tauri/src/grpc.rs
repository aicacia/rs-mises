use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt};
use tauri::{AppHandle, Emitter, async_runtime};
use tokio::sync::{mpsc, oneshot};

use crate::server::ServerMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum GrpcError {
  Internal(String),
}

impl fmt::Display for GrpcError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      GrpcError::Internal(msg) => write!(f, "Internal error: {}", msg),
    }
  }
}

impl From<tauri::Error> for GrpcError {
  fn from(e: tauri::Error) -> Self {
    GrpcError::Internal(e.to_string())
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
  Success {
    request_id: uuid::Uuid,
    data: Frame,
  },
  Error {
    request_id: uuid::Uuid,
    error: GrpcError,
  },
}

#[tauri::command]
pub fn grpc(
  state: tauri::State<'_, mpsc::UnboundedSender<ServerMessage>>,
  app: AppHandle,
  path: String,
  metadata: Metadata,
  body: Vec<u8>,
) -> Result<uuid::Uuid, GrpcError> {
  let request_id = uuid::Uuid::now_v7();

  let (resp_tx, resp_rx) = oneshot::channel::<Result<Vec<u8>, GrpcError>>();

  if let Err(e) = state.send(ServerMessage::ClientRequest {
    id: request_id,
    path,
    metadata,
    body,
    sender: resp_tx,
  }) {
    let _ = app.emit(
      "grpc-response",
      GrpcEvent::Error {
        request_id,
        error: GrpcError::Internal(format!("failed to send request to server: {}", e)),
      },
    );
  }
  let mut header_metadata = Metadata::new();

  header_metadata.insert(
    "content-type".to_string(),
    vec![MetadataValue::Text("application/grpc".to_string())],
  );
  header_metadata.insert(
    "status".to_string(),
    vec![MetadataValue::Text("200".to_string())],
  );

  if let Err(e) = app.emit(
    "grpc-response",
    GrpcEvent::Success {
      request_id,
      data: Frame::Header {
        header: header_metadata.clone(),
      },
    },
  ) {
    let _ = app.emit(
      "grpc-response",
      GrpcEvent::Error {
        request_id,
        error: GrpcError::from(e),
      },
    );
  }

  let _ = async_runtime::spawn(async move {
    match resp_rx.await {
      Ok(Ok(data)) => {
        if let Err(e) = app.emit(
          "grpc-response",
          GrpcEvent::Success {
            request_id,
            data: Frame::Data { data },
          },
        ) {
          let _ = app.emit(
            "grpc-response",
            GrpcEvent::Error {
              request_id,
              error: GrpcError::from(e),
            },
          );
          return;
        }

        let mut trailer_metadata = Metadata::new();
        trailer_metadata.insert(
          "grpc-status".to_string(),
          vec![MetadataValue::Text("200".to_string())],
        );
        if let Err(e) = app.emit(
          "grpc-response",
          GrpcEvent::Success {
            request_id,
            data: Frame::Trailer {
              trailer: trailer_metadata.clone(),
            },
          },
        ) {
          let _ = app.emit(
            "grpc-response",
            GrpcEvent::Error {
              request_id,
              error: GrpcError::from(e),
            },
          );
          return;
        }
      }
      Ok(Err(e)) => {
        let _ = app.emit(
          "grpc-response",
          GrpcEvent::Error {
            request_id,
            error: e,
          },
        );
      }
      Err(_) => {
        let _ = app.emit(
          "grpc-response",
          GrpcEvent::Error {
            request_id,
            error: GrpcError::Internal("server response cancelled".to_string()),
          },
        );
      }
    }
  });

  Ok(request_id)
}
