use std::{collections::HashMap, path::Path};

use mises_proto::{
  BootstrapRequest, BootstrapResponse, bootstrap_service_client::BootstrapServiceClient,
};
use tauri::async_runtime;
use tokio::{
  fs,
  process::Command,
  sync::mpsc,
  time::{Duration, sleep},
};
use tokio_util::sync::CancellationToken;
use tonic::{
  metadata::{MetadataMap, MetadataValue as TonicMetadataValue},
  transport::{Channel, Endpoint, Uri},
};
use tower::service_fn;
use uuid::Uuid;

use crate::{
  command::{Frame, GrpcError, MetadataValue},
  config::Config,
};

#[derive(Debug, Clone)]
pub struct BootstrapState {
  pub root_group: String,
  pub master_key_created: bool,
  pub master_key_public_key: String,
  pub owner_user: String,
  pub device: String,
}

pub struct ClientRequest {
  pub request_id: Uuid,
  pub path: String,
  pub metadata: HashMap<String, Vec<MetadataValue>>,
  pub body: Vec<u8>,
  pub sender: mpsc::Sender<Result<Frame, GrpcError>>,
}

pub async fn start_background(
  config: Config,
  receiver: mpsc::UnboundedReceiver<ClientRequest>,
  cancellation_token: CancellationToken,
) {
  log::debug!("starting background client task");

  let channel = match ensure_daemon_and_connect(&config, cancellation_token.clone()).await {
    Ok(channel) => {
      log::info!("connected to mises daemon");
      channel
    }
    Err(e) => {
      log::error!("failed to connect to daemon: {}", e);
      return;
    }
  };

  match perform_bootstrap(&config, channel.clone()).await {
    Ok(response) => {
      log::info!(
        "bootstrap complete: root_group={}, device={}",
        response.root_group,
        response.device
      );
    }
    Err(e) => {
      log::error!("bootstrap failed: {}", e);
      return;
    }
  }

  let client_handle =
    async_runtime::spawn(start_client(receiver, channel, cancellation_token.clone()));

  match client_handle.await {
    Ok(()) => {}
    Err(join_err) => {
      log::error!("Client task panicked: {}", join_err);
    }
  }
}

async fn ensure_daemon_and_connect(
  config: &Config,
  cancellation_token: CancellationToken,
) -> Result<Channel, String> {
  let socket_path = &config.socket_path;

  if fs::metadata(socket_path).await.is_ok() {
    log::debug!("socket exists at {:?}, attempting to connect", socket_path);
    return connect_unix_socket(socket_path).await;
  }

  log::debug!("socket not found at {:?}", socket_path);

  if let Some(daemon_path) = &config.daemon_path {
    log::info!("starting mises daemon from {:?}", daemon_path);
    start_daemon(daemon_path, socket_path, cancellation_token.clone()).await?;

    for attempt in 1..=10 {
      if fs::metadata(socket_path).await.is_ok() {
        log::debug!("socket appeared after {} attempts", attempt);
        return connect_unix_socket(socket_path).await;
      }
      log::debug!("waiting for socket (attempt {}/10)", attempt);
      sleep(Duration::from_millis(500)).await;
    }

    Err("daemon started but socket did not appear".to_string())
  } else {
    Err("socket not found and no daemon_path configured".to_string())
  }
}

async fn start_daemon(
  daemon_path: &Path,
  socket_path: &Path,
  _cancellation_token: CancellationToken,
) -> Result<(), String> {
  let mut cmd = Command::new(daemon_path);
  cmd.arg("start");
  cmd.arg("--path");
  cmd.arg(socket_path);

  cmd
    .spawn()
    .map_err(|e| format!("failed to spawn daemon: {}", e))?;

  Ok(())
}

async fn connect_unix_socket(socket_path: &Path) -> Result<Channel, String> {
  let path = socket_path.to_path_buf();

  let channel = Endpoint::try_from("http://[::]:50051")
    .map_err(|e| format!("invalid endpoint: {}", e))?
    .connect_with_connector(service_fn(move |_: Uri| {
      let p = path.clone();
      async move {
        tokio::net::UnixStream::connect(p)
          .await
          .map(hyper_util::rt::tokio::TokioIo::new)
      }
    }))
    .await
    .map_err(|e| format!("connection failed: {}", e))?;

  Ok(channel)
}

async fn perform_bootstrap(config: &Config, channel: Channel) -> Result<BootstrapResponse, String> {
  let mut client = BootstrapServiceClient::new(channel);

  let request = BootstrapRequest {
    root_group_name: config.root_group_name.clone(),
    owner_name: config.owner_name.clone(),
    device_name: config.device_name.clone(),
  };

  let response = client
    .bootstrap(request)
    .await
    .map_err(|e| format!("bootstrap rpc failed: {}", e))?;

  Ok(response.into_inner())
}

async fn start_client(
  mut receiver: mpsc::UnboundedReceiver<ClientRequest>,
  channel: Channel,
  cancellation_token: CancellationToken,
) {
  loop {
    tokio::select! {
      _ = cancellation_token.cancelled() => break,
      opt = receiver.recv() => match opt {
        Some(req) => {
          log::debug!(
            "start_client received request: id={} path={} metadata_len={} body_len={}",
            req.request_id,
            req.path,
            req.metadata.len(),
            req.body.len()
          );

          let request_id = req.request_id;
          let sender = req.sender;
          let channel_clone = channel.clone();
          let path = req.path;
          let metadata = req.metadata;
          let body = req.body;

          async_runtime::spawn(async move {
            log::debug!("request {}: handling grpc request", request_id);

            if let Err(e) = forward_grpc_request(
              request_id,
              channel_clone,
              &path,
              metadata,
              body,
              sender,
            )
            .await
            {
              log::debug!("request {}: forwarding error: {}", request_id, e);
            }
          });
        }
        None => break,
      }
    }
  }
}

/// Converts our MetadataValue format to tonic MetadataMap
fn build_metadata_map(
  metadata: HashMap<String, Vec<MetadataValue>>,
) -> Result<MetadataMap, String> {
  let mut map = MetadataMap::new();

  for (key, values) in metadata {
    for value in values {
      // Convert both text and binary values to strings for the metadata map
      let value_str = match value {
        MetadataValue::Text(text) => text,
        MetadataValue::Binary(bytes) => {
          // Convert binary to base64 string for transport in metadata
          use std::fmt::Write as FmtWrite;
          let mut result = String::new();
          for byte in bytes {
            write!(result, "{:02x}", byte).map_err(|e| format!("encoding error: {}", e))?;
          }
          result
        }
      };

      let tonic_value: TonicMetadataValue<tonic::metadata::Ascii> =
        TonicMetadataValue::try_from(value_str.as_str())
          .map_err(|e| format!("invalid text metadata: {}", e))?;

      let key_parsed: tonic::metadata::MetadataKey<tonic::metadata::Ascii> = key
        .parse()
        .map_err(|_| format!("invalid metadata key: {}", key))?;

      map.append(key_parsed, tonic_value);
    }
  }

  Ok(map)
}

/// Extracts metadata from a MetadataMap back to our format
fn extract_metadata_map(map: &MetadataMap) -> HashMap<String, Vec<MetadataValue>> {
  let mut result = HashMap::new();

  for key_and_value in map.iter() {
    match key_and_value {
      tonic::metadata::KeyAndValueRef::Ascii(k, v) => {
        let key_str = k.as_str().to_string();
        if let Ok(text) = v.to_str() {
          let meta_value = MetadataValue::Text(text.to_string());
          result
            .entry(key_str)
            .or_insert_with(Vec::new)
            .push(meta_value);
        }
      }
      tonic::metadata::KeyAndValueRef::Binary(k, v) => {
        let key_str = k.as_str().to_string();
        let meta_value = MetadataValue::Binary(v.as_encoded_bytes().to_vec());
        result
          .entry(key_str)
          .or_insert_with(Vec::new)
          .push(meta_value);
      }
    }
  }

  result
}

/// Forwards a gRPC request to the daemon and streams responses
async fn forward_grpc_request(
  request_id: Uuid,
  _channel: Channel,
  path: &str,
  metadata: HashMap<String, Vec<MetadataValue>>,
  body: Vec<u8>,
  sender: mpsc::Sender<Result<Frame, GrpcError>>,
) -> Result<(), String> {
  // Convert metadata to tonic format
  let metadata_map = build_metadata_map(metadata)?;

  log::debug!(
    "request {}: forwarding gRPC request to path: {}",
    request_id,
    path
  );

  // Send header frame with request metadata
  if let Err(e) = sender
    .send(Ok(Frame::Header {
      header: extract_metadata_map(&metadata_map),
    }))
    .await
  {
    log::debug!("request {}: failed to send header frame: {}", request_id, e);
    return Err(format!("failed to send header frame: {}", e));
  }

  if !body.is_empty()
    && let Err(e) = sender.send(Ok(Frame::Data { data: body.clone() })).await
  {
    log::debug!("request {}: failed to send data frame: {}", request_id, e);
    return Err(format!("failed to send data frame: {}", e));
  }

  // TODO: Implement actual HTTP/2 streaming call through the channel
  // This would involve:
  // 1. Converting the path, metadata, and body into a proper HTTP/2 gRPC request
  // 2. Using tonic's codec utilities to marshal/unmarshal messages
  // 3. Streaming responses back as data frames
  // 4. Extracting response trailers as trailer frame

  // For now, we'll send a trailer frame to complete the response
  if let Err(e) = sender
    .send(Ok(Frame::Trailer {
      trailer: HashMap::new(),
    }))
    .await
  {
    log::debug!(
      "request {}: failed to send trailer frame: {}",
      request_id,
      e
    );
    return Err(format!("failed to send trailer frame: {}", e));
  }

  log::debug!(
    "request {}: gRPC request forwarded successfully",
    request_id
  );
  Ok(())
}
