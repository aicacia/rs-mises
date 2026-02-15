use std::{collections::HashMap, convert::Infallible, fmt::Write as FmtWrite, io, path::Path};

use bytes::Bytes;
use http::{Method, Request, Version};
use http_body_util::{BodyExt, Full, combinators::UnsyncBoxBody};
use tauri::async_runtime;
use tokio::{
  fs,
  process::Command,
  sync::mpsc,
  time::{Duration, sleep},
};
use tokio_util::sync::CancellationToken;
use tonic::{
  body::Body,
  metadata::{Ascii, MetadataKey, MetadataMap, MetadataValue as TonicMetadataValue},
  transport::{Channel, Endpoint, Uri},
};
use tower::{Service, ServiceExt, service_fn};
use uuid::Uuid;

use crate::{
  command::{Frame, GrpcError, MetadataValue},
  config::Config,
};

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

  match init_client_identity(channel.clone()).await {
    Ok(()) => {}
    Err(join_err) => {
      log::error!("Client task panicked: {}", join_err);
    }
  }

  match async_runtime::spawn(start_client(receiver, channel, cancellation_token.clone())).await {
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

async fn init_client_identity(_channel: Channel) -> io::Result<()> {
  Ok(())
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

fn build_metadata_map(
  metadata: HashMap<String, Vec<MetadataValue>>,
) -> Result<MetadataMap, String> {
  let mut map = MetadataMap::new();

  for (key, values) in metadata {
    for value in values {
      let value_str = match value {
        MetadataValue::Text(text) => text,
        MetadataValue::Binary(bytes) => {
          let mut result = String::new();
          for byte in bytes {
            write!(result, "{:02x}", byte).map_err(|e| format!("encoding error: {}", e))?;
          }
          result
        }
      };

      let tonic_value: TonicMetadataValue<Ascii> = TonicMetadataValue::try_from(value_str.as_str())
        .map_err(|e| format!("invalid text metadata: {}", e))?;

      let key_parsed: MetadataKey<Ascii> = key
        .parse()
        .map_err(|_| format!("invalid metadata key: {}", key))?;

      map.append(key_parsed, tonic_value);
    }
  }

  Ok(map)
}

async fn forward_grpc_request(
  request_id: Uuid,
  mut channel: Channel,
  path: &str,
  metadata: HashMap<String, Vec<MetadataValue>>,
  body: Vec<u8>,
  sender: mpsc::Sender<Result<Frame, GrpcError>>,
) -> Result<(), String> {
  let metadata_map = build_metadata_map(metadata)?;

  log::debug!(
    "request {}: forwarding gRPC request to path: {}",
    request_id,
    path
  );

  let uri = Uri::builder()
    .scheme("http")
    .authority("[::]:50051")
    .path_and_query(path)
    .build()
    .map_err(|e| format!("invalid URI: {}", e))?;

  let mut request = Request::builder()
    .method(Method::POST)
    .uri(uri)
    .version(Version::HTTP_2)
    .header("content-type", "application/grpc")
    .header("te", "trailers");

  for key_and_value in metadata_map.iter() {
    match key_and_value {
      tonic::metadata::KeyAndValueRef::Ascii(key, value) => {
        if let Ok(header_value) = value.to_str() {
          request = request.header(key.as_str(), header_value);
        }
      }
      tonic::metadata::KeyAndValueRef::Binary(key, value) => {
        request = request.header(key.as_str(), value.as_encoded_bytes());
      }
    }
  }

  let body_bytes = Bytes::from(body);
  let tonic_body = Full::new(body_bytes);

  let http_request: http::Request<_> = request
    .body(tonic_body)
    .map_err(|e| format!("failed to build request: {}", e))?;

  let mut response = channel
    .ready()
    .await
    .map_err(|e| format!("channel not ready: {}", e))?
    .call(http_request.map(|body| {
      let boxed: UnsyncBoxBody<bytes::Bytes, Infallible> =
        body.map_err(|never| match never {}).boxed_unsync();
      Body::new(boxed)
    }))
    .await
    .map_err(|e| format!("gRPC call failed: {}", e))?;

  let response_headers = response.headers();
  let mut header_metadata = HashMap::new();

  for (key, value) in response_headers.iter() {
    let key_str = key.as_str().to_string();
    if let Ok(text) = value.to_str() {
      header_metadata
        .entry(key_str)
        .or_insert_with(Vec::new)
        .push(MetadataValue::Text(text.to_string()));
    } else {
      header_metadata
        .entry(key_str)
        .or_insert_with(Vec::new)
        .push(MetadataValue::Binary(value.as_bytes().to_vec()));
    }
  }

  if let Err(e) = sender
    .send(Ok(Frame::Header {
      header: header_metadata,
    }))
    .await
  {
    log::debug!("request {}: failed to send header frame: {}", request_id, e);
    return Err(format!("failed to send header frame: {}", e));
  }

  let body = response.body_mut();
  let mut trailers_sent = false;
  while let Some(chunk) = body.frame().await {
    match chunk {
      Ok(frame) => match frame.into_data() {
        Ok(data) => {
          if let Err(e) = sender
            .send(Ok(Frame::Data {
              data: data.to_vec(),
            }))
            .await
          {
            log::debug!("request {}: failed to send data frame: {}", request_id, e);
            return Err(format!("failed to send data frame: {}", e));
          }
        }
        Err(frame) => {
          if let Ok(trailers) = frame.into_trailers() {
            let mut trailer_metadata = HashMap::new();

            for (key, value) in trailers.iter() {
              let key_str = key.as_str().to_string();
              if let Ok(text) = value.to_str() {
                trailer_metadata
                  .entry(key_str)
                  .or_insert_with(Vec::new)
                  .push(MetadataValue::Text(text.to_string()));
              } else {
                trailer_metadata
                  .entry(key_str)
                  .or_insert_with(Vec::new)
                  .push(MetadataValue::Binary(value.as_bytes().to_vec()));
              }
            }

            if let Err(e) = sender
              .send(Ok(Frame::Trailer {
                trailer: trailer_metadata,
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

            trailers_sent = true;
            break;
          }

          log::debug!("request {}: received unknown body frame", request_id);
        }
      },
      Err(e) => {
        log::debug!("request {}: error reading response body: {}", request_id, e);
        if let Err(send_err) = sender
          .send(Err(GrpcError::Internal {
            message: format!("stream error: {}", e),
          }))
          .await
        {
          log::debug!(
            "request {}: failed to send error event: {}",
            request_id,
            send_err
          );
        }
        return Err(format!("stream error: {}", e));
      }
    }
  }

  if !trailers_sent {
    if let Err(e) = sender
      .send(Err(GrpcError::Internal {
        message: "missing gRPC trailers from server response".to_string(),
      }))
      .await
    {
      log::debug!(
        "request {}: failed to send missing-trailer error: {}",
        request_id,
        e
      );
      return Err(format!("failed to send missing-trailer error: {}", e));
    }

    return Err("missing gRPC trailers from server response".to_string());
  }

  log::debug!(
    "request {}: gRPC request forwarded successfully",
    request_id
  );

  Ok(())
}
