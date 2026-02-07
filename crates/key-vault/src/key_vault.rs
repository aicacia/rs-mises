use std::{io, path::PathBuf};

use mises_core::{CoreError, Result, service::graph::KeyVault};
use mises_key::Key;
use tokio::fs;

// Graph-backed key vault implementation helpers
use base64::{Engine, prelude::BASE64_URL_SAFE};
use mises_core::model::{
  keys::KeyMeta,
  node::{NodeMeta, NodeType},
};
use mises_graph::{Element, NodeQuery, Query, field};

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
  async fn get_or_create(&self) -> Result<(Key, Vec<u8>, bool)> {
    match fs::read(&self.path).await {
      Ok(bytes) => Ok((Key::from(bytes.clone()), bytes, false)),
      Err(e) if e.kind() == io::ErrorKind::NotFound => {
        // Generate entropy, derive a normalized seed, persist it, and construct a node-only Key from it
        let mut entropy = [0u8; 32];
        getrandom::getrandom(&mut entropy).map_err(|e| {
          CoreError::other(std::io::Error::other(format!("getrandom error: {}", e)))
        })?;
        // derive normalized seed from the mnemonic so we persist the seed used by `Key::from`
        let mnemonic = bip39::Mnemonic::from_entropy(&entropy).map_err(|e| {
          CoreError::other(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("bip39 error: {}", e),
          ))
        })?;
        let seed = mnemonic.to_seed_normalized("").to_vec();

        if let Some(parent) = self.path.parent() {
          fs::create_dir_all(parent).await.map_err(CoreError::other)?;
        }
        fs::write(&self.path, &seed)
          .await
          .map_err(CoreError::other)?;

        let key = Key::from(seed.clone());
        Ok((key, seed, true))
      }
      Err(e) => Err(CoreError::other(e)),
    }
  }
}

/// A graph-backed KeyVault implementation that will read an existing key node's
/// `private_key` (Base64) if present and return a `Key`. If no key node is
/// present, it will generate a new key and return it (without persisting it).
pub struct GraphKeyVault<R> {
  repo: R,
}

impl<R> GraphKeyVault<R> {
  pub fn new(repo: R) -> Self {
    Self { repo }
  }
}

#[async_trait::async_trait]
impl<R> KeyVault for GraphKeyVault<R>
where
  R: mises_core::traits::Repository + Send + Sync,
{
  async fn get_or_create(&self) -> Result<(Key, Vec<u8>, bool)> {
    // Look for a key node without a derivation path (master key)
    let query = Query::nodes(
      NodeQuery::new(NodeType::Key.as_str()).filter(!field("metadata.derivation_path").exists()),
    );
    let elements = self.repo.query(query).await?;

    for el in elements {
      if let Element::Node(node) = el
        && let NodeMeta::Key(KeyMeta {
          private_key: Some(b64),
          ..
        }) = &node.metadata
      {
        let bytes = BASE64_URL_SAFE.decode(b64.as_bytes()).map_err(|e| {
          CoreError::other(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("base64 decode error: {}", e),
          ))
        })?;
        return Ok((Key::from(bytes.clone()), bytes, false));
      }
    }

    // No existing key found in graph, generate a new node-only key
    let mut entropy = [0u8; 32];
    getrandom::getrandom(&mut entropy)
      .map_err(|e| CoreError::other(std::io::Error::other(format!("getrandom error: {}", e))))?;
    let key = Key::from_entropy(&entropy).map_err(CoreError::from)?;

    Ok((key, entropy.to_vec(), true))
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
    let (k1, seed1, k_created1) = kv.get_or_create().await.unwrap();
    let (k2, seed2, k_created2) = kv.get_or_create().await.unwrap();

    let (_sk1, vk1) = k1.secp256k1_keypair().expect("keypair");
    let (_sk2, vk2) = k2.secp256k1_keypair().expect("keypair");
    let p1 = vk1.to_encoded_point(false);
    let p2 = vk2.to_encoded_point(false);
    assert_eq!(p1.as_bytes(), p2.as_bytes());

    assert!(k_created1);
    assert!(!k_created2);

    // the vault persisted a seed; ensure both seeds are non-empty
    assert!(!seed1.is_empty());
    assert!(!seed2.is_empty());
  }
}
