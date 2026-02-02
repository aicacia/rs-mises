use alloc::{boxed::Box, vec::Vec};
use core::ops::RangeBounds;

use crate::error::GraphError;

#[async_trait::async_trait]
pub trait KeyValueStore: Send + Sync {
  async fn get<K>(&self, key: K) -> Result<Option<Vec<u8>>, GraphError>
  where
    K: AsRef<[u8]> + Send;
  async fn put<K>(&self, key: K, value: Vec<u8>) -> Result<(), GraphError>
  where
    K: AsRef<[u8]> + Send;
  async fn delete<K>(&self, key: K) -> Result<(), GraphError>
  where
    K: AsRef<[u8]> + Send;
  async fn scan<R, F>(&self, range: R, predicate: F) -> Result<Vec<(Vec<u8>, Vec<u8>)>, GraphError>
  where
    R: RangeBounds<Vec<u8>> + Send,
    F: FnMut(&Vec<u8>, &Vec<u8>) -> Option<bool> + Send;

  async fn get_batch<K>(&self, keys: Vec<K>) -> Result<Vec<Option<Vec<u8>>>, GraphError>
  where
    K: AsRef<[u8]> + Send,
  {
    let mut results = Vec::with_capacity(keys.len());
    for key in keys {
      results.push(self.get(key).await?);
    }
    Ok(results)
  }
}
