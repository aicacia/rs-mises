use std::path::PathBuf;

use tauri::{Listener, Manager, async_runtime};
use tauri_plugin_cli::CliExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[cfg(any(windows, target_os = "linux"))]
use tauri_plugin_deep_link::DeepLinkExt;

use crate::{background::start_background, command, config::Config, file_service};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn start() {
  let mut builder = tauri::Builder::default();

  #[cfg(desktop)]
  {
    builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
      if let Some(w) = app.get_webview_window("main") {
        let _r: tauri::Result<()> = w.set_focus();
        let _ = _r;
      }
    }));
  }

  #[cfg(desktop)]
  {
    builder = builder.plugin(tauri_plugin_cli::init());
  }

  let (sender, receiver) = mpsc::unbounded_channel();

  builder
    .manage(sender.clone())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_deep_link::init())
    .plugin(
      tauri_plugin_log::Builder::new()
        .level(tauri_plugin_log::log::LevelFilter::Debug)
        .build(),
    )
    .setup(move |app| {
      #[cfg(all(debug_assertions, desktop))]
      if let Some(window) = app.get_webview_window("main") {
        window.open_devtools();
      }

      #[cfg(any(windows, target_os = "linux"))]
      {
        let _ = app.deep_link().register_all();
      }

      let mut config_path = None;
      match app.cli().matches() {
        Ok(matches) => {
          if let Some(config) = matches.args.get("config")
            && let Some(path) = config.value.as_str()
          {
            config_path = Some(PathBuf::from(path))
          }
        }
        Err(e) => {
          log::error!("Failed to parse CLI arguments: {}", e);
        }
      }

      let config = if let Some(p) = config_path {
        match Config::try_from(p.as_path()) {
          Ok(c) => c,
          Err(e) => {
            log::error!("Failed to load config: {}", e);
            Config::default()
          }
        }
      } else {
        Config::default()
      };

      let cancellation_token = CancellationToken::new();

      if config.file_service.enabled {
        log::info!("file service enabled, starting service");
        let file_service_config = config.file_service.clone();
        let file_service_cancellation = cancellation_token.clone();

        async_runtime::spawn(async move {
          file_service::start_file_service(file_service_config, None, file_service_cancellation).await;
        });
      } else {
        log::info!("file service disabled by configuration");
      }

      let app_cancellation_token = cancellation_token.clone();
      app.once("exit", move |_e| {
        app_cancellation_token.cancel();
      });

      async_runtime::spawn(start_background(config, receiver, cancellation_token));

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![command::grpc])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
