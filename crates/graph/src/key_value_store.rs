use alloc::{boxed::Box, vec::Vec};
use core::ops::RangeBounds;

use crate::error::GraphError;

#[async_trait::async_trait]
pub trait KeyValueStoreExecutor: Send + Sync {
  async fn get<K>(&self, key: K) -> Result<Option<Vec<u8>>, GraphError>
  where
    K: AsRef<[u8]> + Send;
  async fn put<K>(&self, key: K, value: Vec<u8>) -> Result<(), GraphError>
  where
    K: AsRef<[u8]> + Send;
  async fn delete<K>(&self, key: K) -> Result<(), GraphError>
  where
    K: AsRef<[u8]> + Send;
  /// Scan entries in `range`, invoking `predicate` for each entry.
  ///
  /// The `predicate` controls scanning and selection using the following return
  /// values:
  ///
  /// - `Some(true)` — include the `(key, value)` pair in the returned results and continue scanning.
  /// - `Some(false)` — skip the pair and continue scanning.
  /// - `None` — stop scanning early and return the results collected so far.
  ///
  /// Implementations should iterate keys in range order and call `predicate` on
  /// each `(key, value)`. This API allows callers to both filter which entries
  /// are returned and to stop scanning early for efficiency.
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

#[async_trait::async_trait]
pub trait KeyValueStoreTransaction: KeyValueStoreExecutor + Sized {
  async fn commit(self) -> Result<(), GraphError>;
  async fn rollback(self) -> Result<(), GraphError>;
}

#[async_trait::async_trait]
pub trait KeyValueStore: KeyValueStoreExecutor {
  type Transaction: KeyValueStoreTransaction;

  async fn transaction(&self) -> Result<Self::Transaction, GraphError>;
}
