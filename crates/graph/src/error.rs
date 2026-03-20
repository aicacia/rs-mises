use alloc::{
  boxed::Box,
  string::{String, ToString},
};
#[cfg(feature = "std")]
use std::error::Error;

#[cfg(not(feature = "std"))]
use core::fmt::Debug as Error;

#[cfg(feature = "std")]
use std::sync::PoisonError;

#[derive(Debug)]
#[cfg_attr(feature = "rich-errors", derive(thiserror::Error))]
pub enum GraphError {
  #[cfg_attr(feature = "rich-errors", error("not found"))]
  NotFound,
  #[cfg_attr(feature = "rich-errors", error("conflict"))]
  Conflict,
  #[cfg_attr(feature = "rich-errors", error("serialization error: {0}"))]
  SerializationError(String),
  #[cfg(feature = "rich-errors")]
  #[cfg_attr(feature = "rich-errors", error("serialization error: {0}"))]
  Serde(#[cfg_attr(feature = "rich-errors", from)] serde_json::Error),
  #[cfg_attr(feature = "rich-errors", error(transparent))]
  Other(Box<dyn Error + Send + Sync>),
}

#[cfg(not(feature = "rich-errors"))]
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

#[cfg(feature = "std")]
impl std::error::Error for GraphError {}

impl core::fmt::Display for GraphError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      GraphError::NotFound => write!(f, "not found"),
      GraphError::Conflict => write!(f, "conflict"),
      GraphError::SerializationError(s) => write!(f, "serialization error: {}", s),
      #[cfg(feature = "rich-errors")]
      GraphError::Serde(e) => write!(f, "serialization error: {}", e),
      GraphError::Other(e) => write!(f, "{:?}", e),
    }
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
