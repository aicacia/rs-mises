use std::{collections::HashMap, io, sync::Arc};

use mises_core::service::graph::GraphService;
use mises_graph::{InMemoryKeyValueStore, KeyValueRepository, UuidGenerator};

use mises_grpc_server::{BootstrapService, BootstrapServiceServer, proto::FILE_DESCRIPTOR_SET};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use hyper::{
  body::HttpBody,
  header::{CONTENT_TYPE, HeaderName, HeaderValue, TE},
  {Body, Method, Request},
};
use prost::Message;
use tauri::async_runtime;
use tokio::{sync::mpsc, time::timeout};
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;
use uuid::Uuid;

use crate::{
  command::{Frame, GrpcError, MetadataValue},
  in_memory_io::{InMemoryDialer, InMemoryIO},
};
const BASE_URI: &str = "http://[::]:50051";

pub struct ClientRequest {
  pub request_id: Uuid,
  pub path: String,
  pub metadata: HashMap<String, Vec<MetadataValue>>,
  pub body: Vec<u8>,
  pub sender: mpsc::Sender<Result<Frame, GrpcError>>,
}

pub async fn start_background(
  receiver: mpsc::UnboundedReceiver<ClientRequest>,
  cancellation_token: CancellationToken,
) {
  let (io, dialer) = InMemoryIO::new_pair();
  log::debug!("starting background server and client tasks");
  let server_handle = async_runtime::spawn(start_server(io, cancellation_token.clone()));

  if let Ok(fds) = prost_types::FileDescriptorSet::decode(FILE_DESCRIPTOR_SET) {
    log::debug!("Loaded {} file descriptors", fds.file.len());
  } else {
    log::warn!("Failed to parse FILE_DESCRIPTOR_SET");
  }

  let client_handle = async_runtime::spawn(start_client(
    receiver,
    Arc::new(dialer),
    cancellation_token.clone(),
  ));

  match server_handle.await {
    Ok(Ok(())) => {}
    Ok(Err(e)) => {
      log::error!("Server returned error: {}", e);
      cancellation_token.cancel();
    }
    Err(join_err) => {
      log::error!("Server task panicked: {}", join_err);
      cancellation_token.cancel();
    }
  }

  match client_handle.await {
    Ok(()) => {}
    Err(join_err) => {
      log::error!("Client task panicked: {}", join_err);
    }
  }
}

async fn start_client(
  mut receiver: mpsc::UnboundedReceiver<ClientRequest>,
  dialer: Arc<InMemoryDialer>,
  cancellation_token: CancellationToken,
) {
  loop {
    tokio::select! {
      _ = cancellation_token.cancelled() => break,
      opt = receiver.recv() => match opt {
        Some(req) => {
          let dialer = dialer.clone();
          log::debug!("start_client received request: id={} path={} metadata_len={} body_len={}", req.request_id, req.path, req.metadata.len(), req.body.len());
          let request_id = req.request_id;
          let path = req.path;
          let metadata = req.metadata;
          let body = req.body;
          let sender = req.sender;

          async_runtime::spawn(async move {
            log::debug!("request {}: using request body ({} bytes)", request_id, body.len());

            let conn = match dialer.dial() {
              Ok(c) => c,
              Err(e) => {
                log::debug!("request {}: dial error: {}", request_id, e);
                if let Err(send_err) = sender.send(Err(GrpcError::Internal { message: format!("dial error: {}", e) })).await {
                  log::debug!("request {}: failed to send dial error to caller: {}", request_id, send_err);
                }
                return;
              }
            };

            let (mut sender_client, connection) = match hyper::client::conn::handshake(conn).await {
              Ok(pair) => pair,
              Err(e) => {
                log::debug!("request {}: handshake error: {}", request_id, e);
                if let Err(send_err) = sender.send(Err(GrpcError::Internal { message: format!("handshake error: {}", e) })).await {
                  log::debug!("request {}: failed to send handshake error to caller: {}", request_id, send_err);
                }
                return;
              }
            };
            log::debug!("request {}: handshake complete", request_id);

            async_runtime::spawn(async move {
              if let Err(e) = connection.await {
                log::error!("in-memory connection error: {}", e);
              }
            });

            let uri = format!("{BASE_URI}{}", path);
            let mut builder = Request::builder()
              .method(Method::POST)
              .uri(uri)
              .header(CONTENT_TYPE, "application/grpc")
              .header(TE, "trailers");

            // Attach metadata as HTTP/2 headers
            for (k, vals) in metadata.iter() {
              let key = k.to_ascii_lowercase();
              match HeaderName::from_bytes(key.as_bytes()) {
                Ok(name) => {
                  for v in vals {
                    match v {
                      MetadataValue::Text(s) => {
                        if let Ok(val) = HeaderValue::from_str(s) {
                          builder = builder.header(name.clone(), val);
                        } else {
                          log::warn!("invalid metadata text value for {}: {:?}", k, s);
                        }
                      }
                      MetadataValue::Binary(bv) => {
                        if !k.ends_with("-bin") {
                          log::warn!("binary metadata key '{}' recommended to end with '-bin'", k);
                        }
                        let encoded = STANDARD.encode(bv);
                        if let Ok(val) = HeaderValue::from_str(&encoded) {
                          builder = builder.header(name.clone(), val);
                        } else {
                          log::warn!("invalid metadata binary value for {} (base64): {:?}", k, encoded);
                        }
                      }
                    }
                  }
                }
                Err(e) => {
                  log::warn!("invalid metadata key '{}': {}", k, e);
                }
              }
            }

            let request = match builder.body(Body::from(body)) {
              Ok(r) => r,
              Err(e) => {
                log::debug!("request {}: failed to build request: {}", request_id, e);
                if let Err(send_err) = sender.send(Err(GrpcError::Internal { message: format!("failed to build request: {}", e) })).await {
                  log::debug!("request {}: failed to send build-request error to caller: {}", request_id, send_err);
                }
                return;
              }
            };

            log::debug!("request {}: sending HTTP/2 request to {}", request_id, path);
            let response = match timeout(std::time::Duration::from_secs(10), sender_client.send_request(request)).await {
              Ok(Ok(res)) => {
                log::debug!("request {}: received response (status={})", request_id, res.status());
                res
              }
              Ok(Err(e)) => {
                log::debug!("request {}: request error: {}", request_id, e);
                if let Err(send_err) = sender.send(Err(GrpcError::Internal { message: format!("request error: {}", e) })).await {
                  log::debug!("request {}: failed to send request error to caller: {}", request_id, send_err);
                }
                return;
              }
              Err(_) => {
                log::debug!("request {}: rpc timed out", request_id);
                if let Err(send_err) = sender.send(Err(GrpcError::Internal { message: "rpc timed out".into() })).await {
                  log::debug!("request {}: failed to send rpc timeout error to caller: {}", request_id, send_err);
                }
                return;
              }
            };

            // Convert initial HTTP headers into a Header frame and send it before reading body
            {
              let mut hdrs: HashMap<String, Vec<MetadataValue>> = HashMap::new();
              for (name, value) in response.headers().iter() {
                log::debug!("request {}: response header {}: {:?}", request_id, name.as_str(), value);
                let key = name.as_str().to_string();
                let entry = hdrs.entry(key.clone()).or_default();
                if key.ends_with("-bin") {
                  match value.to_str() {
                    Ok(s) => match STANDARD.decode(s) {
                      Ok(decoded) => entry.push(MetadataValue::Binary(decoded)),
                      Err(e) => {
                        log::warn!("invalid base64 for binary header {}: {}", name, e);
                      }
                    },
                    Err(_) => {
                      log::warn!("non-utf8 value for binary header {}", name);
                    }
                  }
                } else {
                  let text = match value.to_str() {
                    Ok(s) => s.to_string(),
                    Err(_) => String::from_utf8_lossy(value.as_bytes()).to_string(),
                  };
                  entry.push(MetadataValue::Text(text));
                }
              }

              if !hdrs.is_empty() {
                log::debug!("request {}: sending header frame ({} headers)", request_id, hdrs.len());
                if let Err(send_err) = sender.send(Ok(Frame::Header { header: hdrs })).await {
                  log::debug!("request {}: failed to send header frame to caller: {}", request_id, send_err);
                }
              }
            }

            // Read the response body manually so we can retrieve HTTP trailers afterwards
            let mut body = response.into_body();
            let mut bytes = Vec::new();
            while let Some(chunk_res) = body.data().await {
              match chunk_res {
                Ok(chunk) => bytes.extend_from_slice(&chunk),
                Err(e) => {
                  log::debug!("request {}: failed to read response body: {}", request_id, e);

                  // Emit a trailer frame with grpc-status and grpc-message so the client
                  // receives a proper gRPC status via trailers instead of an opaque error
                  let mut trails: HashMap<String, Vec<MetadataValue>> = HashMap::new();
                  trails.insert(
                    "grpc-status".to_string(),
                    vec![MetadataValue::Text("13".to_string())],
                  );
                  trails.insert(
                    "grpc-message".to_string(),
                    vec![MetadataValue::Text(format!("failed to read response body: {}", e))],
                  );

                  if let Err(send_err) = sender.send(Ok(Frame::Trailer { trailer: trails })).await {
                    log::debug!("request {}: failed to send trailer (body-read error) to caller: {}", request_id, send_err);
                  }

                  return;
                }
              }
            }
            log::debug!("request {}: read {} body bytes", request_id, bytes.len());

            if bytes.len() < 5 {
              log::debug!("request {}: grpc response too short ({} bytes)", request_id, bytes.len());

              let mut trails: HashMap<String, Vec<MetadataValue>> = HashMap::new();
              trails.insert("grpc-status".to_string(), vec![MetadataValue::Text("13".to_string())]);
              trails.insert(
                "grpc-message".to_string(),
                vec![MetadataValue::Text("grpc response too short".to_string())],
              );

              if let Err(send_err) = sender.send(Ok(Frame::Trailer { trailer: trails })).await {
                log::debug!("request {}: failed to send grpc-too-short trailer to caller: {}", request_id, send_err);
              }

              return;
            }

            if bytes[0] != 0u8 {
              log::debug!("request {}: compressed responses not supported", request_id);

              let mut trails: HashMap<String, Vec<MetadataValue>> = HashMap::new();
              trails.insert("grpc-status".to_string(), vec![MetadataValue::Text("13".to_string())]);
              trails.insert(
                "grpc-message".to_string(),
                vec![MetadataValue::Text("compressed responses not supported".to_string())],
              );

              if let Err(send_err) = sender.send(Ok(Frame::Trailer { trailer: trails })).await {
                log::debug!("request {}: failed to send compression-unsupported trailer to caller: {}", request_id, send_err);
              }

              return;
            }

            let len = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
            if bytes.len() < 5 + len {
              log::debug!("request {}: grpc response length mismatch: expected {}, got {}", request_id, len, bytes.len() - 5);

              let mut trails: HashMap<String, Vec<MetadataValue>> = HashMap::new();
              trails.insert(
                "grpc-status".to_string(),
                vec![MetadataValue::Text("13".to_string())],
              );
              trails.insert(
                "grpc-message".to_string(),
                vec![MetadataValue::Text(format!("grpc response length mismatch: expected {}, got {}", len, bytes.len() - 5))],
              );

              if let Err(send_err) = sender.send(Ok(Frame::Trailer { trailer: trails })).await {
                log::debug!("request {}: failed to send length-mismatch trailer to caller: {}", request_id, send_err);
              }

              return;
            }

            let msg = bytes[5..5 + len].to_vec();
            // Reconstruct gRPC wire frame: compression flag (0) + 4-byte big-endian length + payload
            let mut framed_msg = Vec::with_capacity(5 + msg.len());
            framed_msg.push(0u8);
            framed_msg.extend_from_slice(&(msg.len() as u32).to_be_bytes());
            framed_msg.extend_from_slice(&msg);
            log::debug!("request {}: sending data frame ({} bytes) first_bytes={:?}", request_id, framed_msg.len(), &framed_msg[..std::cmp::min(8, framed_msg.len())]);
            if let Err(send_err) = sender.send(Ok(Frame::Data { data: framed_msg })).await {
              log::debug!("request {}: failed to send data frame to caller: {}", request_id, send_err);
            }

            match body.trailers().await {
              Ok(Some(trailer_map)) => {
                log::debug!("request {}: received trailers ({} entries)", request_id, trailer_map.iter().count());
                let mut trails: HashMap<String, Vec<MetadataValue>> = HashMap::new();
                for (name, value) in trailer_map.iter() {
                  log::debug!("request {}: trailer {}: {:?}", request_id, name.as_str(), value);
                  let key = name.as_str().to_string();
                  let entry = trails.entry(key.clone()).or_default();
                  if key.ends_with("-bin") {
                    match value.to_str() {
                      Ok(s) => match STANDARD.decode(s) {
                        Ok(decoded) => entry.push(MetadataValue::Binary(decoded)),
                        Err(e) => log::warn!("invalid base64 for trailer {}: {}", name, e),
                      },
                      Err(_) => log::warn!("non-utf8 value for trailer {}", name),
                    }
                  } else {
                    let text = match value.to_str() {
                      Ok(s) => s.to_string(),
                      Err(_) => String::from_utf8_lossy(value.as_bytes()).to_string(),
                    };
                    entry.push(MetadataValue::Text(text));
                  }
                }

                if !trails.is_empty() {
                  log::debug!("request {}: sending trailer frame ({} trailers)", request_id, trails.len());
                  if let Err(send_err) = sender.send(Ok(Frame::Trailer { trailer: trails })).await {
                    log::debug!("request {}: failed to send trailer frame to caller: {}", request_id, send_err);
                  }
                }
              }
              Ok(None) => {
                log::debug!("request {}: no trailers present", request_id);
              }
              Err(e) => {
                log::warn!("failed to read trailers: {}", e);
              }
            }
          });
        }
        None => break,
      }
    }
  }
}

async fn start_server(io: InMemoryIO, cancellation_token: CancellationToken) -> io::Result<()> {
  let repo = KeyValueRepository::new(InMemoryKeyValueStore::default(), UuidGenerator::new());

  let graph_service = GraphService::new(repo);

  let reflection_service = tonic_reflection::server::Builder::configure()
    .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
    .build_v1()
    .map_err(|e| io::Error::other(format!("failed to build reflection service: {}", e)))?;

  Server::builder()
    .add_service(BootstrapServiceServer::new(BootstrapService::new(
      graph_service,
    )))
    .add_service(reflection_service)
    .serve_with_incoming_shutdown(io, cancellation_token.cancelled())
    .await
    .map_err(|e| io::Error::other(format!("server error: {}", e)))?;

  Ok(())
}
