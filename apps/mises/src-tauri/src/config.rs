use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
  pub socket_path: PathBuf,
  pub daemon_path: Option<PathBuf>,
  #[serde(default = "default_grpc_port")]
  pub grpc_port: u16,
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
