#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

extern crate alloc;

pub mod edge;
pub mod error;
pub mod key_value_repository;
pub mod node;
pub mod query;
pub mod repository;
pub mod types;
#[cfg(feature = "uuid")]
pub mod uuid_generator;

#[cfg(feature = "uuid")]
pub use crate::uuid_generator::UuidGenerator;
pub use crate::{
  edge::Edge,
  error::GraphError,
  key_value_repository::{IdGenerator, KeyValueRepository, KeyValueTransaction},
  node::Node,
  query::{
    ComparisonOp, EdgeDirection, EdgeQuery, Field, Filter, NodeQuery, Predicate, Query,
    QueryOptions, field,
  },
  repository::{Executor, Repository, Transaction},
  types::{Element, Id, Value},
};
pub use mises_async_kv_bytes::{KeyValueStore, KeyValueStoreExecutor, KeyValueStoreTransaction};

#[cfg(feature = "in-memory")]
mod in_memory_wrapper {
  use alloc::{boxed::Box, vec::Vec};
  use core::ops::RangeBounds;

  use super::*;

  #[derive(Clone)]
  pub struct InMemoryKeyValueStore {
    inner: mises_async_kv_bytes::InMemoryKeyValueStore,
  }

  impl InMemoryKeyValueStore {
    pub fn new() -> Self {
      Self {
        inner: mises_async_kv_bytes::InMemoryKeyValueStore::new(),
      }
    }
  }

  impl Default for InMemoryKeyValueStore {
    fn default() -> Self {
      Self::new()
    }
  }

  #[async_trait::async_trait]
  impl KeyValueStoreExecutor for InMemoryKeyValueStore {
    type Error = GraphError;

    async fn get<K>(&self, key: K) -> Result<Option<Vec<u8>>, Self::Error>
    where
      K: AsRef<[u8]> + Send,
    {
      self.inner.get(key).await.map_err(GraphError::from)
    }

    async fn put<K>(&self, key: K, value: Vec<u8>) -> Result<(), Self::Error>
    where
      K: AsRef<[u8]> + Send,
    {
      self.inner.put(key, value).await.map_err(GraphError::from)
    }

    async fn delete<K>(&self, key: K) -> Result<(), Self::Error>
    where
      K: AsRef<[u8]> + Send,
    {
      self.inner.delete(key).await.map_err(GraphError::from)
    }

    async fn scan<R, F>(&self, range: R, f: F) -> Result<(), Self::Error>
    where
      R: RangeBounds<Vec<u8>> + Send,
      F: FnMut(&Vec<u8>, &Vec<u8>) -> bool + Send,
    {
      self.inner.scan(range, f).await.map_err(GraphError::from)
    }

    async fn get_batch<K>(&self, keys: Vec<K>) -> Result<Vec<Option<Vec<u8>>>, Self::Error>
    where
      K: AsRef<[u8]> + Send,
    {
      self.inner.get_batch(keys).await.map_err(GraphError::from)
    }
  }

  pub struct InMemoryTransaction {
    inner: mises_async_kv_bytes::InMemoryTransaction,
  }

  #[async_trait::async_trait]
  impl KeyValueStoreExecutor for InMemoryTransaction {
    type Error = GraphError;

    async fn get<K>(&self, key: K) -> Result<Option<Vec<u8>>, Self::Error>
    where
      K: AsRef<[u8]> + Send,
    {
      self.inner.get(key).await.map_err(GraphError::from)
    }

    async fn put<K>(&self, key: K, value: Vec<u8>) -> Result<(), Self::Error>
    where
      K: AsRef<[u8]> + Send,
    {
      self.inner.put(key, value).await.map_err(GraphError::from)
    }

    async fn delete<K>(&self, key: K) -> Result<(), Self::Error>
    where
      K: AsRef<[u8]> + Send,
    {
      self.inner.delete(key).await.map_err(GraphError::from)
    }

    async fn scan<R, F>(&self, range: R, f: F) -> Result<(), Self::Error>
    where
      R: RangeBounds<Vec<u8>> + Send,
      F: FnMut(&Vec<u8>, &Vec<u8>) -> bool + Send,
    {
      self.inner.scan(range, f).await.map_err(GraphError::from)
    }

    async fn get_batch<K>(&self, keys: Vec<K>) -> Result<Vec<Option<Vec<u8>>>, Self::Error>
    where
      K: AsRef<[u8]> + Send,
    {
      self.inner.get_batch(keys).await.map_err(GraphError::from)
    }
  }

  #[async_trait::async_trait]
  impl KeyValueStoreTransaction for InMemoryTransaction {
    async fn commit(self) -> Result<(), Self::Error> {
      self.inner.commit().await.map_err(GraphError::from)
    }

    async fn rollback(self) -> Result<(), Self::Error> {
      self.inner.rollback().await.map_err(GraphError::from)
    }
  }

  #[async_trait::async_trait]
  impl KeyValueStore for InMemoryKeyValueStore {
    type Transaction = InMemoryTransaction;

    async fn transaction(&self) -> Result<Self::Transaction, Self::Error> {
      let tx = self.inner.transaction().await.map_err(GraphError::from)?;
      Ok(InMemoryTransaction { inner: tx })
    }
  }

  #[async_trait::async_trait]
  impl KeyValueStore for InMemoryTransaction {
    type Transaction = Self;

    async fn transaction(&self) -> Result<Self::Transaction, Self::Error> {
      let tx = self.inner.transaction().await.map_err(GraphError::from)?;
      Ok(InMemoryTransaction { inner: tx })
    }
  }
}

#[cfg(feature = "in-memory")]
pub use in_memory_wrapper::{InMemoryKeyValueStore, InMemoryTransaction};

#[cfg(feature = "in-memory")]
/// Helper alias for a `KeyValueRepository` where all underlying stores are
/// `InMemoryKeyValueStore`.
///
/// Usage: `InMemoryKeyValueRepository<Id, Model, Props, IdGen>`
pub type InMemoryKeyValueRepository<I, M, P, G> = crate::key_value_repository::KeyValueRepository<
  I,
  M,
  P,
  G,
  InMemoryKeyValueStore,
  InMemoryKeyValueStore,
  InMemoryKeyValueStore,
  InMemoryKeyValueStore,
>;
