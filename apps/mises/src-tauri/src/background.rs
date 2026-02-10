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
use tonic::transport::{Channel, Endpoint, Uri};
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
  _channel: Channel,
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

          async_runtime::spawn(async move {
            log::debug!("request {}: handling grpc request", request_id);

            let err = GrpcError::Internal {
              message: "gRPC forwarding not yet implemented".to_string(),
            };

            if let Err(send_err) = sender.send(Err(err)).await {
              log::debug!("request {}: failed to send error to caller: {}", request_id, send_err);
            }
          });
        }
        None => break,
      }
    }
  }
}
