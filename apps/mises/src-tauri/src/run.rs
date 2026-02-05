use tauri::{Listener, RunEvent, async_runtime};
use tokio::sync::{broadcast::Receiver, mpsc};
use tokio_util::sync::CancellationToken;

use crate::{
  grpc,
  server::{ServerMessage, run_server},
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let (sender, receiver): (
    mpsc::UnboundedSender<ServerMessage>,
    mpsc::UnboundedReceiver<ServerMessage>,
  ) = mpsc::unbounded_channel();

  tauri::Builder::default()
    .manage(sender.clone())
    .setup(move |app| {
      let cancellation_token = CancellationToken::new();

      let app_cancellation_token = cancellation_token.clone();
      app.once("exit", move |_e| {
        app_cancellation_token.cancel();
      });

      async_runtime::spawn(run_server(receiver, cancellation_token));
      Ok(())
    })
    .plugin(
      tauri_plugin_log::Builder::new()
        .level(tauri_plugin_log::log::LevelFilter::Debug)
        .build(),
    )
    .invoke_handler(tauri::generate_handler![grpc::grpc])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
