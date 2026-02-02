use core::{error::Error, fmt};

#[cfg(not(feature = "std"))]
use alloc::{
  boxed::Box,
  string::{String, ToString},
};
#[cfg(feature = "std")]
use std::{
  boxed::Box,
  string::{String, ToString},
};

#[derive(Debug)]
pub enum GraphError {
  NotFound,
  Conflict,
  SerializationError(String),
  Other(Box<dyn Error + Send + Sync>),
}

impl fmt::Display for GraphError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      GraphError::NotFound => write!(f, "not found"),
      GraphError::Conflict => write!(f, "conflict"),
      GraphError::SerializationError(e) => write!(f, "serialization error: {}", e),
      GraphError::Other(e) => write!(f, "other error: {}", e),
    }
  }
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

#[cfg(feature = "std")]
impl<T> From<std::sync::PoisonError<T>> for GraphError {
  fn from(e: std::sync::PoisonError<T>) -> Self {
    GraphError::Other(e.to_string().into())
  }
}

impl Error for GraphError {}
