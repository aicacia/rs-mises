use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileServiceConfig {
  #[serde(default)]
  pub enabled: bool,
  #[serde(default = "default_file_service_root")]
  pub root_dir: PathBuf,
  #[serde(default = "default_file_service_bind_host")]
  pub bind_host: String,
  #[serde(default = "default_file_service_bind_port")]
  pub bind_port: u16,
}

fn default_file_service_root() -> PathBuf {
  PathBuf::from("./data/file-service")
}

fn default_file_service_bind_host() -> String {
  "127.0.0.1".to_string()
}

fn default_file_service_bind_port() -> u16 {
  9000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
  pub socket_path: PathBuf,
  pub daemon_path: Option<PathBuf>,
  #[serde(default = "default_grpc_port")]
  pub grpc_port: u16,
  #[serde(default)]
  pub file_service: FileServiceConfig,
}

fn default_grpc_port() -> u16 {
  50051
}

impl Default for Config {
  fn default() -> Self {
    Self {
      socket_path: PathBuf::from("../../../mises.sock"),
      daemon_path: None,
      grpc_port: 50051,
      file_service: FileServiceConfig {
        enabled: false,
        root_dir: default_file_service_root(),
        bind_host: default_file_service_bind_host(),
        bind_port: default_file_service_bind_port(),
      },
    }
  }
}

impl<'a> TryFrom<&'a Path> for Config {
  type Error = config::ConfigError;

  fn try_from(config_path: &'a Path) -> Result<Self, Self::Error> {
    config::Config::builder()
      .add_source(config::File::with_name(
        config_path.to_string_lossy().as_ref(),
      ))
      .build()?
      .try_deserialize()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_config_has_file_service_disabled() {
    let config = Config::default();
    assert!(!config.file_service.enabled);
    assert_eq!(config.file_service.bind_host, "127.0.0.1");
    assert_eq!(config.file_service.bind_port, 9000);
  }
}
