#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

#[cfg(feature = "in-memory")]
pub mod in_memory_key_value_store;
pub mod key_value_store;

pub use key_value_store::{KeyValueStore, KeyValueStoreExecutor, KeyValueStoreTransaction};

#[cfg(feature = "in-memory")]
pub use in_memory_key_value_store::{InMemoryKeyValueStore, InMemoryTransaction};
