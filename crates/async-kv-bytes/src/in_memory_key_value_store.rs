use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, vec::Vec};
use core::ops::RangeBounds;
#[cfg(feature = "std")]
use std::sync::RwLock;

#[cfg(not(feature = "std"))]
use spin::RwLock;

use crate::{KeyValueStore, KeyValueStoreExecutor, KeyValueStoreTransaction};

#[derive(Clone, Debug)]
pub struct InMemoryKeyValueStore {
  inner: Arc<RwLock<BTreeMap<Vec<u8>, Vec<u8>>>>,
}

pub struct InMemoryTransaction {
  snapshot: RwLock<BTreeMap<Vec<u8>, Vec<u8>>>,
  original: Arc<RwLock<BTreeMap<Vec<u8>, Vec<u8>>>>,
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
  #[cfg(feature = "std")]
  PoisonedLock,
}

impl core::fmt::Display for InMemoryError {
  fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      #[cfg(feature = "std")]
      InMemoryError::PoisonedLock => write!(_f, "poisoned lock"),
      #[cfg(not(feature = "std"))]
      _ => unreachable!(),
    }
  }
}

impl core::error::Error for InMemoryError {}

#[cfg(feature = "std")]
impl<T> From<std::sync::PoisonError<T>> for InMemoryError {
  fn from(_e: std::sync::PoisonError<T>) -> Self {
    Self::PoisonedLock
  }
}

#[async_trait::async_trait]
impl KeyValueStoreExecutor for InMemoryKeyValueStore {
  type Error = InMemoryError;

  async fn get<K>(&self, key: K) -> Result<Option<Vec<u8>>, Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    #[cfg(feature = "std")]
    let guard = self.inner.read()?;
    #[cfg(not(feature = "std"))]
    let guard = self.inner.read();

    Ok(guard.get(key.as_ref()).cloned())
  }

  async fn put<K>(&self, key: K, value: Vec<u8>) -> Result<(), Self::Error>
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

  async fn delete<K>(&self, key: K) -> Result<(), Self::Error>
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

  async fn scan<R, F>(&self, range: R, mut f: F) -> Result<(), Self::Error>
  where
    R: RangeBounds<Vec<u8>> + Send,
    F: FnMut(&Vec<u8>, &Vec<u8>) -> bool + Send,
  {
    #[cfg(feature = "std")]
    let guard = self.inner.read()?;
    #[cfg(not(feature = "std"))]
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

    #[cfg(feature = "std")]
    let guard = self.inner.read()?;
    #[cfg(not(feature = "std"))]
    let guard = self.inner.read();

    for key in keys {
      results.push(guard.get(key.as_ref()).cloned());
    }

    Ok(results)
  }
}

#[async_trait::async_trait]
impl KeyValueStore for InMemoryKeyValueStore {
  type Transaction = InMemoryTransaction;

  async fn transaction(&self) -> Result<Self::Transaction, Self::Error> {
    #[cfg(feature = "std")]
    let guard = self.inner.read()?;
    #[cfg(not(feature = "std"))]
    let guard = self.inner.read();

    Ok(InMemoryTransaction {
      snapshot: RwLock::new(guard.clone()),
      original: self.inner.clone(),
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
    #[cfg(feature = "std")]
    let guard = self.snapshot.read()?;
    #[cfg(not(feature = "std"))]
    let guard = self.snapshot.read();

    Ok(guard.get(key.as_ref()).cloned())
  }

  async fn put<K>(&self, key: K, value: Vec<u8>) -> Result<(), Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    #[cfg(feature = "std")]
    let mut guard = self.snapshot.write()?;
    #[cfg(not(feature = "std"))]
    let mut guard = self.snapshot.write();

    guard.insert(key.as_ref().to_vec(), value);
    Ok(())
  }

  async fn delete<K>(&self, key: K) -> Result<(), Self::Error>
  where
    K: AsRef<[u8]> + Send,
  {
    #[cfg(feature = "std")]
    let mut guard = self.snapshot.write()?;
    #[cfg(not(feature = "std"))]
    let mut guard = self.snapshot.write();

    guard.remove(key.as_ref());
    Ok(())
  }

  async fn scan<R, F>(&self, range: R, mut predicate: F) -> Result<(), Self::Error>
  where
    R: RangeBounds<Vec<u8>> + Send,
    F: FnMut(&Vec<u8>, &Vec<u8>) -> bool + Send,
  {
    #[cfg(feature = "std")]
    let guard = self.snapshot.read()?;
    #[cfg(not(feature = "std"))]
    let guard = self.snapshot.read();

    for (key, value) in guard.range(range) {
      if !predicate(key, value) {
        break;
      }
    }

    Ok(())
  }
}

#[async_trait::async_trait]
impl KeyValueStoreTransaction for InMemoryTransaction {
  async fn commit(self) -> Result<(), Self::Error> {
    #[cfg(feature = "std")]
    let mut original = self.original.write()?;
    #[cfg(not(feature = "std"))]
    let mut original = self.original.write();

    #[cfg(feature = "std")]
    let snapshot = self.snapshot.read()?;
    #[cfg(not(feature = "std"))]
    let snapshot = self.snapshot.read();

    *original = snapshot.clone();
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
    #[cfg(feature = "std")]
    let guard = self.snapshot.read()?;
    #[cfg(not(feature = "std"))]
    let guard = self.snapshot.read();

    Ok(InMemoryTransaction {
      snapshot: RwLock::new(guard.clone()),
      original: self.original.clone(),
    })
  }
}
