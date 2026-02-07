use tauri::{Listener, async_runtime};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{background::start_background, command};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn start() {
  let (sender, receiver) = mpsc::unbounded_channel();

  tauri::Builder::default()
    .manage(sender.clone())
    .setup(move |app| {
      let cancellation_token = CancellationToken::new();

      let app_cancellation_token = cancellation_token.clone();
      app.once("exit", move |_e| {
        app_cancellation_token.cancel();
      });

      async_runtime::spawn(start_background(receiver, cancellation_token));
      Ok(())
    })
    .plugin(
      tauri_plugin_log::Builder::new()
        .level(tauri_plugin_log::log::LevelFilter::Debug)
        .build(),
    )
    .invoke_handler(tauri::generate_handler![command::grpc])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
