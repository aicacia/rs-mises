use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, vec::Vec};
use core::{error::Error, fmt, ops::RangeBounds};

use parking_lot::RwLock;

use crate::{KeyValueStore, KeyValueStoreExecutor, KeyValueStoreTransaction};

#[derive(Clone, Debug)]
pub struct InMemoryKeyValueStore {
  inner: Arc<RwLock<BTreeMap<Vec<u8>, Vec<u8>>>>,
}

pub struct InMemoryTransaction {
  original: Arc<RwLock<BTreeMap<Vec<u8>, Vec<u8>>>>,
  changes: RwLock<BTreeMap<Vec<u8>, Option<Vec<u8>>>>,
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

#[derive(Debug)]
pub enum InMemoryError {
  UnexpectedError,
}

impl fmt::Display for InMemoryError {
  fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::UnexpectedError => write!(_f, "An unexpected error occurred"),
    }
  }
}

impl Error for InMemoryError {}

#[async_trait::async_trait]
impl KeyValueStoreExecutor for InMemoryKeyValueStore {
  type Error = InMemoryError;

  async fn get<K>(&self, key: K) -> Result<Option<Vec<u8>>, Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    let guard = self.inner.read();

    Ok(guard.get(key.as_ref()).cloned())
  }

  async fn put<K>(&self, key: K, value: Vec<u8>) -> Result<(), Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    let mut guard = self.inner.write();

    guard.insert(key.as_ref().to_vec(), value);
    Ok(())
  }

  async fn delete<K>(&self, key: K) -> Result<(), Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    let mut guard = self.inner.write();

    guard.remove(key.as_ref());
    Ok(())
  }

  async fn scan<R, F>(&self, range: R, mut f: F) -> Result<(), Self::Error>
  where
    R: RangeBounds<Vec<u8>> + Send,
    F: FnMut(&Vec<u8>, &Vec<u8>) -> bool + Send,
  {
    let guard = self.inner.read();

    for (key, value) in guard.range(range) {
      if !f(key, value) {
        break;
      }
    }

    Ok(())
  }

  async fn get_batch<K>(&self, keys: Vec<K>) -> Result<Vec<Option<Vec<u8>>>, Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    let mut results = Vec::with_capacity(keys.len());

    let guard = self.inner.read();

    for key in keys {
      results.push(guard.get(key.as_ref()).cloned());
    }

    Ok(results)
  }

  async fn put_batch<K>(&self, entries: Vec<(K, Vec<u8>)>) -> Result<(), Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    let mut guard = self.inner.write();

    for (key, value) in entries {
      guard.insert(key.as_ref().to_vec(), value);
    }

    Ok(())
  }

  async fn delete_batch<K>(&self, keys: Vec<K>) -> Result<(), Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    let mut guard = self.inner.write();

    for key in keys {
      guard.remove(key.as_ref());
    }

    Ok(())
  }
}

#[async_trait::async_trait]
impl KeyValueStore for InMemoryKeyValueStore {
  type Transaction = InMemoryTransaction;

  async fn transaction(&self) -> Result<Self::Transaction, Self::Error> {
    Ok(InMemoryTransaction {
      original: self.inner.clone(),
      changes: RwLock::new(BTreeMap::new()),
    })
  }
}

#[async_trait::async_trait]
impl KeyValueStoreExecutor for InMemoryTransaction {
  type Error = InMemoryError;

  async fn get<K>(&self, key: K) -> Result<Option<Vec<u8>>, Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    let key_ref = key.as_ref();
    {
      let changes = self.changes.read();

      if let Some(change) = changes.get(key_ref) {
        return Ok(change.clone());
      }
    }

    let original = self.original.read();

    Ok(original.get(key_ref).cloned())
  }

  async fn put<K>(&self, key: K, value: Vec<u8>) -> Result<(), Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    let mut changes = self.changes.write();

    changes.insert(key.as_ref().to_vec(), Some(value));
    Ok(())
  }

  async fn delete<K>(&self, key: K) -> Result<(), Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    let mut changes = self.changes.write();

    changes.insert(key.as_ref().to_vec(), None);
    Ok(())
  }

  async fn scan<R, F>(&self, range: R, mut f: F) -> Result<(), Self::Error>
  where
    R: RangeBounds<Vec<u8>> + Send,
    F: FnMut(&Vec<u8>, &Vec<u8>) -> bool + Send,
  {
    let original = self.original.read();
    let changes = self.changes.read();

    for (key, value) in original.range(range) {
      if let Some(change) = changes.get(key) {
        if let Some(v) = change
          && !f(key, v)
        {
          break;
        }
      } else if !f(key, value) {
        break;
      }
    }

    Ok(())
  }

  async fn get_batch<K>(&self, keys: Vec<K>) -> Result<Vec<Option<Vec<u8>>>, Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    let mut results = Vec::with_capacity(keys.len());

    let original = self.original.read();
    let changes = self.changes.read();

    for key in keys {
      let key_ref = key.as_ref();

      if let Some(change) = changes.get(key_ref) {
        results.push(change.clone());
      } else {
        results.push(original.get(key_ref).cloned());
      }
    }

    Ok(results)
  }

  async fn put_batch<K>(&self, entries: Vec<(K, Vec<u8>)>) -> Result<(), Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    let mut changes = self.changes.write();

    for (key, value) in entries {
      changes.insert(key.as_ref().to_vec(), Some(value));
    }

    Ok(())
  }

  async fn delete_batch<K>(&self, keys: Vec<K>) -> Result<(), Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    let mut changes = self.changes.write();

    for key in keys {
      changes.insert(key.as_ref().to_vec(), None);
    }

    Ok(())
  }
}

#[async_trait::async_trait]
impl KeyValueStoreTransaction for InMemoryTransaction {
  async fn commit(self) -> Result<(), Self::Error> {
    let mut original = self.original.write();
    let changes = self.changes.into_inner();

    for (key, value) in changes.into_iter() {
      match value {
        Some(v) => {
          original.insert(key.clone(), v.clone());
        }
        None => {
          original.remove(&key);
        }
      }
    }

    Ok(())
  }

  async fn rollback(self) -> Result<(), Self::Error> {
    Ok(())
  }
}

#[async_trait::async_trait]
impl KeyValueStore for InMemoryTransaction {
  type Transaction = Self;

  async fn transaction(&self) -> Result<Self::Transaction, Self::Error> {
    Ok(InMemoryTransaction {
      original: self.original.clone(),
      changes: RwLock::new(BTreeMap::new()),
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use alloc::vec;
  use core::ops::Bound;

  #[tokio::test]
  async fn test_put_and_get() {
    let store = InMemoryKeyValueStore::new();
    let key = b"test_key";
    let value = vec![1, 2, 3, 4, 5];

    store.put(key, value.clone()).await.unwrap();
    let result = store.get(key).await.unwrap();

    assert_eq!(result, Some(value));
  }

  #[tokio::test]
  async fn test_get_nonexistent_key() {
    let store = InMemoryKeyValueStore::new();
    let result = store.get(b"nonexistent").await.unwrap();

    assert_eq!(result, None);
  }

  #[tokio::test]
  async fn test_delete() {
    let store = InMemoryKeyValueStore::new();
    let key = b"delete_key";
    let value = vec![1, 2, 3];

    store.put(key, value).await.unwrap();
    assert!(store.get(key).await.unwrap().is_some());

    store.delete(key).await.unwrap();
    assert_eq!(store.get(key).await.unwrap(), None);
  }

  #[tokio::test]
  async fn test_delete_nonexistent_key() {
    let store = InMemoryKeyValueStore::new();
    let result = store.delete(b"nonexistent").await;

    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_overwrite_value() {
    let store = InMemoryKeyValueStore::new();
    let key = b"key";
    let value1 = vec![1, 2, 3];
    let value2 = vec![4, 5, 6];

    store.put(key, value1).await.unwrap();
    assert_eq!(store.get(key).await.unwrap(), Some(vec![1, 2, 3]));

    store.put(key, value2).await.unwrap();
    assert_eq!(store.get(key).await.unwrap(), Some(vec![4, 5, 6]));
  }

  #[tokio::test]
  async fn test_get_batch() {
    let store = InMemoryKeyValueStore::new();
    let keys = vec![b"key1".to_vec(), b"key2".to_vec(), b"key3".to_vec()];
    let values: alloc::vec::Vec<alloc::vec::Vec<u8>> = [vec![1, 2], vec![3, 4], vec![5, 6]].into();

    for (key, value) in keys.iter().zip(values.iter()) {
      store.put(key.clone(), value.clone()).await.unwrap();
    }

    let results = store.get_batch(keys).await.unwrap();

    assert_eq!(results.len(), 3);
    assert_eq!(results[0], Some(vec![1, 2]));
    assert_eq!(results[1], Some(vec![3, 4]));
    assert_eq!(results[2], Some(vec![5, 6]));
  }

  #[tokio::test]
  async fn test_get_batch_with_missing_keys() {
    let store = InMemoryKeyValueStore::new();

    store.put(b"key1", vec![1, 2]).await.unwrap();
    store.put(b"key3", vec![5, 6]).await.unwrap();

    let keys = vec![b"key1".to_vec(), b"key2".to_vec(), b"key3".to_vec()];
    let results = store.get_batch(keys).await.unwrap();

    assert_eq!(results.len(), 3);
    assert_eq!(results[0], Some(vec![1, 2]));
    assert_eq!(results[1], None);
    assert_eq!(results[2], Some(vec![5, 6]));
  }

  #[tokio::test]
  async fn test_put_batch() {
    let store = InMemoryKeyValueStore::new();
    let entries = vec![
      (b"key1".to_vec(), vec![1, 2]),
      (b"key2".to_vec(), vec![3, 4]),
      (b"key3".to_vec(), vec![5, 6]),
    ];

    store.put_batch(entries).await.unwrap();

    assert_eq!(store.get(b"key1").await.unwrap(), Some(vec![1, 2]));
    assert_eq!(store.get(b"key2").await.unwrap(), Some(vec![3, 4]));
    assert_eq!(store.get(b"key3").await.unwrap(), Some(vec![5, 6]));
  }

  #[tokio::test]
  async fn test_delete_batch() {
    let store = InMemoryKeyValueStore::new();

    store.put(b"key1", vec![1]).await.unwrap();
    store.put(b"key2", vec![2]).await.unwrap();
    store.put(b"key3", vec![3]).await.unwrap();

    let keys = vec![b"key1".to_vec(), b"key2".to_vec()];
    store.delete_batch(keys).await.unwrap();

    assert_eq!(store.get(b"key1").await.unwrap(), None);
    assert_eq!(store.get(b"key2").await.unwrap(), None);
    assert_eq!(store.get(b"key3").await.unwrap(), Some(vec![3]));
  }

  #[tokio::test]
  async fn test_scan_all() {
    let store = InMemoryKeyValueStore::new();

    store.put(b"key1", vec![1]).await.unwrap();
    store.put(b"key2", vec![2]).await.unwrap();
    store.put(b"key3", vec![3]).await.unwrap();

    let mut scanned = Vec::new();
    store
      .scan::<(Bound<Vec<u8>>, Bound<Vec<u8>>), _>((Bound::Unbounded, Bound::Unbounded), |k, v| {
        scanned.push((k.clone(), v.clone()));
        true
      })
      .await
      .unwrap();

    assert_eq!(scanned.len(), 3);
  }

  #[tokio::test]
  async fn test_scan_early_termination() {
    let store = InMemoryKeyValueStore::new();

    store.put(b"key1", vec![1]).await.unwrap();
    store.put(b"key2", vec![2]).await.unwrap();
    store.put(b"key3", vec![3]).await.unwrap();

    let mut scanned = Vec::new();
    store
      .scan::<(Bound<Vec<u8>>, Bound<Vec<u8>>), _>((Bound::Unbounded, Bound::Unbounded), |k, v| {
        scanned.push((k.clone(), v.clone()));
        scanned.len() < 2
      })
      .await
      .unwrap();

    assert_eq!(scanned.len(), 2);
  }

  #[tokio::test]
  async fn test_scan_range() {
    let store = InMemoryKeyValueStore::new();

    store.put(b"a", vec![1]).await.unwrap();
    store.put(b"b", vec![2]).await.unwrap();
    store.put(b"c", vec![3]).await.unwrap();
    store.put(b"d", vec![4]).await.unwrap();

    let mut scanned = Vec::new();
    store
      .scan::<_, _>(
        (
          Bound::Included(b"b".to_vec()),
          Bound::Included(b"c".to_vec()),
        ),
        |k, v| {
          scanned.push((k.clone(), v.clone()));
          true
        },
      )
      .await
      .unwrap();

    assert_eq!(scanned.len(), 2);
    assert_eq!(scanned[0].0, b"b");
    assert_eq!(scanned[1].0, b"c");
  }

  #[tokio::test]
  async fn test_transaction_commit() {
    let store = InMemoryKeyValueStore::new();

    store.put(b"key1", vec![1, 2]).await.unwrap();

    let txn = store.transaction().await.unwrap();
    txn.put(b"key2", vec![3, 4]).await.unwrap();
    txn.delete(b"key1").await.unwrap();
    txn.commit().await.unwrap();

    assert_eq!(store.get(b"key1").await.unwrap(), None);
    assert_eq!(store.get(b"key2").await.unwrap(), Some(vec![3, 4]));
  }

  #[tokio::test]
  async fn test_transaction_rollback() {
    let store = InMemoryKeyValueStore::new();

    store.put(b"key1", vec![1, 2]).await.unwrap();

    let txn = store.transaction().await.unwrap();
    txn.put(b"key2", vec![3, 4]).await.unwrap();
    txn.delete(b"key1").await.unwrap();
    txn.rollback().await.unwrap();

    assert_eq!(store.get(b"key1").await.unwrap(), Some(vec![1, 2]));
    assert_eq!(store.get(b"key2").await.unwrap(), None);
  }

  #[tokio::test]
  async fn test_transaction_reads_uncommitted_changes() {
    let store = InMemoryKeyValueStore::new();

    store.put(b"key1", vec![1, 2]).await.unwrap();

    let txn = store.transaction().await.unwrap();
    txn.put(b"key2", vec![3, 4]).await.unwrap();

    let result = txn.get(b"key2").await.unwrap();
    assert_eq!(result, Some(vec![3, 4]));

    let result = txn.get(b"key1").await.unwrap();
    assert_eq!(result, Some(vec![1, 2]));
  }

  #[tokio::test]
  async fn test_transaction_isolated_from_store() {
    let store = InMemoryKeyValueStore::new();

    store.put(b"key1", vec![1, 2]).await.unwrap();

    let txn = store.transaction().await.unwrap();
    txn.put(b"key1", vec![9, 9]).await.unwrap();

    let result = store.get(b"key1").await.unwrap();
    assert_eq!(result, Some(vec![1, 2]));

    let txn_result = txn.get(b"key1").await.unwrap();
    assert_eq!(txn_result, Some(vec![9, 9]));

    txn.commit().await.unwrap();

    let result = store.get(b"key1").await.unwrap();
    assert_eq!(result, Some(vec![9, 9]));
  }

  #[tokio::test]
  async fn test_transaction_delete_then_put() {
    let store = InMemoryKeyValueStore::new();

    store.put(b"key1", vec![1, 2]).await.unwrap();

    let txn = store.transaction().await.unwrap();
    txn.delete(b"key1").await.unwrap();
    txn.put(b"key1", vec![3, 4]).await.unwrap();

    assert_eq!(txn.get(b"key1").await.unwrap(), Some(vec![3, 4]));

    txn.commit().await.unwrap();

    assert_eq!(store.get(b"key1").await.unwrap(), Some(vec![3, 4]));
  }

  #[tokio::test]
  async fn test_transaction_batch_operations() {
    let store = InMemoryKeyValueStore::new();

    let txn = store.transaction().await.unwrap();
    let entries = vec![
      (b"k1".to_vec(), vec![1]),
      (b"k2".to_vec(), vec![2]),
      (b"k3".to_vec(), vec![3]),
    ];
    txn.put_batch(entries).await.unwrap();

    txn.commit().await.unwrap();

    assert_eq!(store.get(b"k1").await.unwrap(), Some(vec![1]));
    assert_eq!(store.get(b"k2").await.unwrap(), Some(vec![2]));
    assert_eq!(store.get(b"k3").await.unwrap(), Some(vec![3]));
  }

  #[tokio::test]
  async fn test_nested_transaction() {
    let store = InMemoryKeyValueStore::new();

    store.put(b"key1", vec![1, 2]).await.unwrap();

    let txn1 = store.transaction().await.unwrap();
    txn1.put(b"key2", vec![2, 3]).await.unwrap();

    let txn2 = txn1.transaction().await.unwrap();
    txn2.put(b"key3", vec![3, 4]).await.unwrap();

    txn2.commit().await.unwrap();
    txn1.commit().await.unwrap();

    assert_eq!(store.get(b"key1").await.unwrap(), Some(vec![1, 2]));
    assert_eq!(store.get(b"key2").await.unwrap(), Some(vec![2, 3]));
    assert_eq!(store.get(b"key3").await.unwrap(), Some(vec![3, 4]));
  }

  #[tokio::test]
  async fn test_clone_independence() {
    let store1 = InMemoryKeyValueStore::new();
    store1.put(b"key1", vec![1, 2]).await.unwrap();

    let store2 = store1.clone();
    store2.put(b"key2", vec![3, 4]).await.unwrap();

    assert_eq!(store1.get(b"key1").await.unwrap(), Some(vec![1, 2]));
    assert_eq!(store1.get(b"key2").await.unwrap(), Some(vec![3, 4]));

    assert_eq!(store2.get(b"key1").await.unwrap(), Some(vec![1, 2]));
    assert_eq!(store2.get(b"key2").await.unwrap(), Some(vec![3, 4]));
  }

  #[tokio::test]
  async fn test_empty_store() {
    let store = InMemoryKeyValueStore::new();

    let mut count = 0;
    store
      .scan::<(Bound<Vec<u8>>, Bound<Vec<u8>>), _>((Bound::Unbounded, Bound::Unbounded), |_, _| {
        count += 1;
        true
      })
      .await
      .unwrap();

    assert_eq!(count, 0);
  }

  #[tokio::test]
  async fn test_default_constructor() {
    let store = InMemoryKeyValueStore::default();
    assert_eq!(store.get(b"any_key").await.unwrap(), None);
  }
}
