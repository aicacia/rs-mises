use alloc::{
  boxed::Box,
  string::{String, ToString},
};
use core::error::Error;

#[cfg(feature = "std")]
use std::sync::PoisonError;

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
  #[error("not found")]
  NotFound,
  #[error("conflict")]
  Conflict,
  #[error("serialization error: {0}")]
  SerializationError(String),
  #[error(transparent)]
  Other(Box<dyn Error + Send + Sync>),
}

impl From<serde_json::Error> for GraphError {
  fn from(e: serde_json::Error) -> Self {
    GraphError::SerializationError(e.to_string())
  }
}

impl From<Box<dyn Error + Send + Sync + 'static>> for GraphError {
  fn from(e: Box<dyn Error + Send + Sync + 'static>) -> Self {
    GraphError::Other(e)
  }
}

#[cfg(feature = "in-memory")]
impl From<mises_async_kv_bytes::in_memory_key_value_store::InMemoryKeyValueError> for GraphError {
  fn from(e: mises_async_kv_bytes::in_memory_key_value_store::InMemoryKeyValueError) -> Self {
    GraphError::Other(Box::new(e))
  }
}

#[cfg(feature = "std")]
impl<T> From<PoisonError<T>> for GraphError {
  fn from(e: PoisonError<T>) -> Self {
    GraphError::Other(e.to_string().into())
  }
}
