use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
  pub log_level: String,
  pub socket_path: PathBuf,
  pub master_key_path: PathBuf,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      log_level: "DEBUG".to_owned(),
      socket_path: PathBuf::from("./mises.sock"),
      master_key_path: PathBuf::from("./master.key"),
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
