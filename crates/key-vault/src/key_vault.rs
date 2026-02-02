use std::{io, path::PathBuf};

use mises_core::{CoreError, Result, service::graph::KeyVault};
use mises_key::MasterKey;
use tokio::fs;

#[derive(Clone)]
pub struct FileKeyVault {
  path: PathBuf,
}

impl FileKeyVault {
  pub fn new(path: PathBuf) -> Self {
    Self { path }
  }
}

#[async_trait::async_trait]
impl KeyVault for FileKeyVault {
  async fn get_or_create(&self) -> Result<(MasterKey, bool)> {
    match fs::read(&self.path).await {
      Ok(bytes) => Ok((MasterKey::from(bytes), false)),
      Err(e) if e.kind() == io::ErrorKind::NotFound => {
        let master_key = MasterKey::new_master().map_err(CoreError::from)?;
        if let Some(parent) = self.path.parent() {
          fs::create_dir_all(parent).await.map_err(CoreError::other)?;
        }
        fs::write(&self.path, master_key.as_bytes())
          .await
          .map_err(CoreError::other)?;
        Ok((master_key, true))
      }
      Err(e) => Err(CoreError::other(e)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use uuid::Uuid;

  #[tokio::test]
  async fn roundtrip_store() {
    let tmp = std::env::temp_dir().join(format!("mkv-{}", Uuid::now_v7()));
    let kv = FileKeyVault::new(tmp.clone());
    let (mk1, mk_created1) = kv.get_or_create().await.unwrap();
    let (mk2, mk_created2) = kv.get_or_create().await.unwrap();
    assert_eq!(mk1.as_bytes(), mk2.as_bytes());
    assert!(mk_created1);
    assert!(!mk_created2);
  }
}
