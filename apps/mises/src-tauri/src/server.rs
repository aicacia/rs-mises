use std::{collections::HashMap, io};

use mises_core::service::graph::GraphService;
use mises_graph::{InMemoryKeyValueStore, KeyValueRepository, UuidGenerator};
use mises_grpc_server::{BootstrapService, BootstrapServiceServer, proto::FILE_DESCRIPTOR_SET};
use mises_key_vault::FileKeyVault;
use tauri::async_runtime;
use tokio::sync::{mpsc, oneshot::Sender};
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;

use crate::{
  grpc::{GrpcError, MetadataValue},
  in_memory_io::InMemoryIO,
};

pub enum ServerMessage {
  ClientRequest {
    id: uuid::Uuid,
    path: String,
    metadata: HashMap<String, Vec<MetadataValue>>,
    body: Vec<u8>,
    sender: Sender<Result<Vec<u8>, GrpcError>>,
  },
  Close,
}

pub async fn run_server(
  mut receiver: mpsc::UnboundedReceiver<ServerMessage>,
  cancellation_token: CancellationToken,
) {
  let io: InMemoryIO = InMemoryIO::new();

  let serve_handle = async_runtime::spawn(serve(io, cancellation_token.clone()));

  while let Some(msg) = receiver.recv().await {
    match msg {
      ServerMessage::Close => {
        println!("Server is closing...");
        break;
      }
      ServerMessage::ClientRequest {
        id,
        path,
        metadata,
        body,
        sender,
      } => {
        if let Err(_) = sender.send(Ok(Vec::new())) {
          log::error!("Failed to send response to client");
        }
      }
    }
  }

  if let Err(e) = serve_handle.await {
    log::error!("Server task failed: {}", e);
  }
}

pub async fn serve(io: InMemoryIO, cancellation_token: CancellationToken) -> io::Result<()> {
  let repo = KeyValueRepository::new(InMemoryKeyValueStore::default(), UuidGenerator::new());
  let key_vault = FileKeyVault::new("./master.key".into());

  let graph_service = GraphService::new(repo, key_vault);

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
