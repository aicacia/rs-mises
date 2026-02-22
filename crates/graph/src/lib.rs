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

  use super::{GraphError, KeyValueStore, KeyValueStoreExecutor, KeyValueStoreTransaction};

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
mod in_memory_repository {
  use core::marker::PhantomData;

  use crate::{Id, IdGenerator, Value, in_memory_wrapper::InMemoryKeyValueStore};

  /// Concrete implementation of `KeyValueRepositoryStore` for in-memory storage.
  #[derive(Clone)]
  pub struct InMemoryRepositoryStore<I, M, P, G>
  where
    I: Id,
    M: Value,
    P: Value,
    G: IdGenerator<I>,
  {
    node_store: InMemoryKeyValueStore,
    edge_store: InMemoryKeyValueStore,
    from_index_store: InMemoryKeyValueStore,
    to_index_store: InMemoryKeyValueStore,
    id_gen: G,
    _phantom: PhantomData<(I, M, P)>,
  }

  impl<I, M, P, G> InMemoryRepositoryStore<I, M, P, G>
  where
    I: Id,
    M: Value,
    P: Value,
    G: IdGenerator<I>,
  {
    pub fn new(id_gen: G) -> Self {
      Self {
        node_store: InMemoryKeyValueStore::new(),
        edge_store: InMemoryKeyValueStore::new(),
        from_index_store: InMemoryKeyValueStore::new(),
        to_index_store: InMemoryKeyValueStore::new(),
        id_gen,
        _phantom: PhantomData,
      }
    }
  }

  impl<I, M, P, G> crate::key_value_repository::KeyValueRepositoryStore
    for InMemoryRepositoryStore<I, M, P, G>
  where
    I: Id,
    M: Value,
    P: Value,
    G: IdGenerator<I> + 'static,
  {
    type Id = I;
    type NodeMeta = M;
    type EdgeProps = P;
    type Store = InMemoryKeyValueStore;
    type IdGen = G;

    fn node_store(&self) -> &Self::Store {
      &self.node_store
    }

    fn edge_store(&self) -> &Self::Store {
      &self.edge_store
    }

    fn from_index_store(&self) -> &Self::Store {
      &self.from_index_store
    }

    fn to_index_store(&self) -> &Self::Store {
      &self.to_index_store
    }

    fn id_gen(&self) -> &Self::IdGen {
      &self.id_gen
    }
  }
}

#[cfg(feature = "in-memory")]
pub use in_memory_repository::InMemoryRepositoryStore;

#[cfg(feature = "in-memory")]
impl<I, M, P, G> KeyValueRepository<InMemoryRepositoryStore<I, M, P, G>>
where
  I: Id,
  M: Value,
  P: Value,
  G: IdGenerator<I> + 'static,
{
  /// Create a new in-memory repository with the given ID generator.
  pub fn new_in_memory(id_gen: G) -> Self {
    let store = InMemoryRepositoryStore::new(id_gen);
    Self::new(store)
  }
}

#[cfg(feature = "in-memory")]
/// Helper alias for an in-memory `KeyValueRepository`.
///
/// Usage: `InMemoryKeyValueRepository::<IdType, MetaType, PropsType, GeneratorType>::new_in_memory(gen)`
pub type InMemoryKeyValueRepository<I, M, P, G> =
  KeyValueRepository<InMemoryRepositoryStore<I, M, P, G>>;
