use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, vec::Vec};
use core::ops::RangeBounds;
#[cfg(feature = "std")]
use std::sync::RwLock;

#[cfg(not(feature = "std"))]
use spin::RwLock;

use crate::{GraphError, KeyValueStore};

#[derive(Clone, Debug)]
pub struct InMemoryKeyValueStore {
  inner: Arc<RwLock<BTreeMap<Vec<u8>, Vec<u8>>>>,
}

impl From<BTreeMap<Vec<u8>, Vec<u8>>> for InMemoryKeyValueStore {
  fn from(map: BTreeMap<Vec<u8>, Vec<u8>>) -> Self {
    Self {
      inner: Arc::new(RwLock::new(map)),
    }
  }
}

impl Default for InMemoryKeyValueStore {
  fn default() -> Self {
    Self::from(BTreeMap::new())
  }
}

impl InMemoryKeyValueStore {
  pub fn new() -> Self {
    Self::default()
  }
}

#[async_trait::async_trait]
impl KeyValueStore for InMemoryKeyValueStore {
  async fn get<K>(&self, key: K) -> Result<Option<Vec<u8>>, GraphError>
  where
    K: AsRef<[u8]> + Send,
  {
    #[cfg(feature = "std")]
    let guard = self.inner.read()?;
    #[cfg(not(feature = "std"))]
    let guard = self.inner.read();

    Ok(guard.get(key.as_ref()).cloned())
  }

  async fn put<K>(&self, key: K, value: Vec<u8>) -> Result<(), GraphError>
  where
    K: AsRef<[u8]> + Send,
  {
    #[cfg(feature = "std")]
    let mut guard = self.inner.write()?;
    #[cfg(not(feature = "std"))]
    let mut guard = self.inner.write();

    guard.insert(key.as_ref().to_vec(), value);
    Ok(())
  }

  async fn delete<K>(&self, key: K) -> Result<(), GraphError>
  where
    K: AsRef<[u8]> + Send,
  {
    #[cfg(feature = "std")]
    let mut guard = self.inner.write()?;
    #[cfg(not(feature = "std"))]
    let mut guard = self.inner.write();

    guard.remove(key.as_ref());
    Ok(())
  }

  async fn scan<R, F>(
    &self,
    range: R,
    mut predicate: F,
  ) -> Result<Vec<(Vec<u8>, Vec<u8>)>, GraphError>
  where
    R: RangeBounds<Vec<u8>> + Send,
    F: FnMut(&Vec<u8>, &Vec<u8>) -> Option<bool> + Send,
  {
    #[cfg(feature = "std")]
    let guard = self.inner.read()?;
    #[cfg(not(feature = "std"))]
    let guard = self.inner.read();

    let mut results = Vec::new();

    for (key, value) in guard.range(range) {
      match predicate(key, value) {
        Some(true) => results.push((key.clone(), value.clone())),
        Some(false) => {}
        None => break,
      }
    }

    Ok(results)
  }
}
