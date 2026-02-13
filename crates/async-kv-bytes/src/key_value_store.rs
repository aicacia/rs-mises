use alloc::{boxed::Box, vec::Vec};
use core::{error::Error, ops::RangeBounds};

#[async_trait::async_trait]
pub trait KeyValueStoreExecutor: Send + Sync {
  type Error: Error;

  async fn get<K>(&self, key: K) -> Result<Option<Vec<u8>>, Self::Error>
  where
    K: AsRef<[u8]> + Send;
  async fn put<K>(&self, key: K, value: Vec<u8>) -> Result<(), Self::Error>
  where
    K: AsRef<[u8]> + Send;
  async fn delete<K>(&self, key: K) -> Result<(), Self::Error>
  where
    K: AsRef<[u8]> + Send;

  /// Scans the key-value store for entries within the specified key range,
  /// if the function returns `true`, the scan will continue, otherwise it will stop
  /// if it returns `false`, the scan will stop immediately
  async fn scan<R, F>(&self, range: R, f: F) -> Result<(), Self::Error>
  where
    R: RangeBounds<Vec<u8>> + Send,
    F: FnMut(&Vec<u8>, &Vec<u8>) -> bool + Send;

  async fn get_batch<K>(&self, keys: Vec<K>) -> Result<Vec<Option<Vec<u8>>>, Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    let mut results = Vec::with_capacity(keys.len());
    for key in keys {
      results.push(self.get(key).await?);
    }
    Ok(results)
  }

  async fn put_batch<K>(&self, entries: Vec<(K, Vec<u8>)>) -> Result<(), Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    for (key, value) in entries {
      self.put(key, value).await?;
    }
    Ok(())
  }

  async fn delete_batch<K>(&self, keys: Vec<K>) -> Result<(), Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    for key in keys {
      self.delete(key).await?;
    }
    Ok(())
  }
}

#[async_trait::async_trait]
pub trait KeyValueStoreTransaction: KeyValueStoreExecutor + Sized {
  async fn commit(self) -> Result<(), Self::Error>;
  async fn rollback(self) -> Result<(), Self::Error>;
}

#[async_trait::async_trait]
pub trait KeyValueStore: KeyValueStoreExecutor {
  type Transaction: KeyValueStoreTransaction;

  async fn transaction(&self) -> Result<Self::Transaction, Self::Error>;
}
