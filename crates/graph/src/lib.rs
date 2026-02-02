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

pub use crate::edge::*;
pub use crate::error::*;
#[cfg(feature = "in-memory")]
pub use crate::in_memory_key_value_store::*;
pub use crate::key_value_repository::*;
pub use crate::key_value_store::*;
pub use crate::node::*;
pub use crate::query::*;
pub use crate::repository::*;
pub use crate::types::*;
#[cfg(feature = "uuid")]
pub use crate::uuid_generator::*;
