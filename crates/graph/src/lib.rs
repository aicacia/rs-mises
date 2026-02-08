#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

extern crate alloc;

pub mod edge;
pub mod error;
#[cfg(feature = "in-memory")]
pub mod in_memory_key_value_store;
pub mod key_value_repository;
pub mod key_value_store;
pub mod node;
pub mod query;
pub mod repository;
pub mod types;
#[cfg(feature = "uuid")]
pub mod uuid_generator;

pub use crate::edge::Edge;
pub use crate::error::GraphError;
#[cfg(feature = "in-memory")]
pub use crate::in_memory_key_value_store::{InMemoryKeyValueStore, InMemoryTransaction};
pub use crate::key_value_repository::{IdGenerator, KeyValueRepository, KeyValueTransaction};
pub use crate::key_value_store::{KeyValueStore, KeyValueStoreExecutor, KeyValueStoreTransaction};
pub use crate::node::Node;
pub use crate::query::{
  ComparisonOp, EdgeDirection, EdgeQuery, Field, Filter, NodeQuery, Predicate, Query, QueryOptions,
  field,
};
pub use crate::repository::{Executor, Repository, Transaction};
pub use crate::types::{Element, Id, Value};
#[cfg(feature = "uuid")]
pub use crate::uuid_generator::UuidGenerator;
