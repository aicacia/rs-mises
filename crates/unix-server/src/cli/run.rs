use std::{io, os::unix::fs::FileTypeExt, path::Path, str::FromStr, sync::Arc};

use crate::{
  cli::args::{CliArgs, CliCommand},
  config::Config,
};
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use mises_core::service::graph::{BootstrapOptionsBuilder, GraphService};
use mises_graph::{InMemoryKeyValueStore, KeyValueRepository, UuidGenerator};
use mises_grpc_server::{
  BootstrapService, OidcService, bootstrap_service_server::BootstrapServiceServer,
  oidc_service_server::OidcServiceServer, proto::FILE_DESCRIPTOR_SET,
};

use tokio::{fs, net::UnixListener};
use tokio_stream::wrappers::UnixListenerStream;
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;
use tracing_subscriber::layer::SubscriberExt;

pub async fn run() -> io::Result<()> {
  let args = CliArgs::parse();
  let mut config = Config::try_from(Path::new(&args.config)).unwrap_or_default();

  tracing_log::LogTracer::init()
    .map_err(|e| io::Error::other(format!("failed to init log tracer: {}", e)))?;

  let level = tracing::Level::from_str(&config.log_level).unwrap_or(tracing::Level::DEBUG);
  let subscriber = tracing_subscriber::registry()
    .with(
      tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        format!(
          "{level},h2=warn,axum::rejection=trace",
          level = level.as_str().to_lowercase()
        )
        .into()
      }),
    )
    .with(tracing_subscriber::fmt::layer());
  tracing::subscriber::set_global_default(subscriber)
    .map_err(|e| io::Error::other(format!("failed to set tracing subscriber: {}", e)))?;

  let cancellation_token = CancellationToken::new();

  let mut command_handle = match args.command {
    Some(CliCommand::Completions { shell }) => tokio::task::spawn_blocking(move || {
      generate(
        shell,
        &mut CliCommand::command(),
        env!("CARGO_CRATE_NAME"),
        &mut io::stdout(),
      );
      Ok(())
    }),
    Some(CliCommand::Start { start }) => {
      if let Some(path) = start.path {
        log::info!("overriding socket path to {:?}", path);
        config.socket_path = path;
      }
      tokio::spawn(serve(Arc::new(config), cancellation_token.child_token()))
    }
    None => tokio::spawn(serve(Arc::new(config), cancellation_token.child_token())),
  };

  tokio::select! {
    result = &mut command_handle => {
      match result {
        Ok(Ok(_)) => log::debug!("server shutdown complete"),
        Ok(Err(e)) => log::error!("command error: {}", e),
        Err(e) => log::error!("join error: {}", e),
      }
    }
    _ = shutdown_signal(cancellation_token.clone()) => {
      let wait_duration = std::time::Duration::from_secs(5);
      log::debug!("shutdown initiated, waiting up to {}s for server to stop", wait_duration.as_secs());

      match tokio::time::timeout(wait_duration, &mut command_handle).await {
        Ok(result) => {
          match result {
            Ok(Ok(_)) => log::debug!("server shutdown complete"),
            Ok(Err(e)) => log::error!("command error: {}", e),
            Err(e) => log::error!("join error: {}", e),
          }
        }
        Err(_) => {
          log::warn!("server did not stop within timeout, aborting");
          command_handle.abort();
          match command_handle.await {
            Ok(Ok(_)) => log::debug!("server shutdown complete after abort"),
            Ok(Err(e)) => log::error!("command error after abort: {}", e),
            Err(e) => log::error!("join error after abort: {}", e),
          }
        }
      }

      log::debug!("shutdown complete");
    }
  }

  Ok(())
}

async fn serve(config: Arc<Config>, cancellation_token: CancellationToken) -> io::Result<()> {
  let bind_path = &config.socket_path;

  if fs::metadata(&bind_path).await.is_ok() {
    let _ = fs::remove_file(&bind_path).await;
  }
  if let Some(parent) = bind_path.parent() {
    fs::create_dir_all(parent).await?;
  }

  let uds = UnixListener::bind(bind_path)?;
  let uds_stream = UnixListenerStream::new(uds);

  let store = InMemoryKeyValueStore::new();
  let repo = KeyValueRepository::new(store.clone(), UuidGenerator::new());

  let graph_service = GraphService::new(repo.clone());

  graph_service
    .bootstrap(
      BootstrapOptionsBuilder::new()
        .root_group_name(config.root_group_name.clone())
        .owner_name(config.owner_name.clone())
        .device_name(config.device_name.clone())
        .now(chrono::Utc::now())
        .build(),
    )
    .await
    .map_err(|e| io::Error::other(format!("failed to bootstrap graph service: {}", e)))?;

  let reflection_service = tonic_reflection::server::Builder::configure()
    .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
    .build_v1()
    .map_err(|e| io::Error::other(format!("failed to build reflection service: {}", e)))?;

  let server_result = Server::builder()
    .add_service(BootstrapServiceServer::new(BootstrapService::new(
      repo.clone(),
    )))
    .add_service(OidcServiceServer::new(OidcService::new(
      repo.clone(),
      store.clone(),
      bind_path.to_string_lossy().to_string(),
      None,
      config.sign_in_url.clone(),
    )))
    .add_service(reflection_service)
    .serve_with_incoming_shutdown(uds_stream, cancellation_token.cancelled())
    .await;

  match fs::symlink_metadata(&bind_path).await {
    Ok(meta) => {
      if meta.file_type().is_socket() {
        match fs::remove_file(&bind_path).await {
          Ok(_) => log::debug!("removed socket file {:?}", bind_path),
          Err(e) => log::warn!("failed to remove socket file {:?}: {}", bind_path, e),
        }
      } else {
        log::debug!("{:?} exists but is not a socket; not removing", bind_path);
      }
    }
    Err(e) => {
      if e.kind() != io::ErrorKind::NotFound {
        log::warn!("failed to stat socket path {:?}: {}", bind_path, e);
      }
    }
  }

  server_result.map_err(|e| io::Error::other(format!("unix serve error: {}", e)))?;

  Ok(())
}

async fn shutdown_signal(cancellation_token: CancellationToken) {
  let ctrl_c = async { tokio::signal::ctrl_c().await };

  let terminate = async {
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
      Ok(mut signal) => match signal.recv().await {
        Some(_) => Ok(()),
        None => Ok(()),
      },
      Err(e) => Err(e),
    }
  };

  let result = tokio::select! {
    result = ctrl_c => result,
    result = terminate => result,
  };

  match result {
    Ok(_) => log::debug!("shutdown signal received, shutting down"),
    Err(e) => log::error!("error receiving shutdown signal: {}", e),
  }

  cancellation_token.cancel();
}
