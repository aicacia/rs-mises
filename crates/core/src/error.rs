use alloc::{boxed::Box, string::String};
use core::error::Error;

use serde_json::Error as SerdeError;

use mises_graph::GraphError;
use mises_key::KeyError;

#[derive(Debug, thiserror::Error)]
pub enum InvalidInput {
  #[error("failed to serialize {0}")]
  SerializationFailed(String),
  #[error("{0}")]
  Other(String),
}

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
  #[error("graph error: {0}")]
  Graph(#[from] GraphError),
  #[error("key error: {0}")]
  Key(#[from] KeyError),
  #[error("serde error: {0}")]
  Serde(#[from] SerdeError),
  #[error("not found")]
  NotFound,
  #[error("forbidden")]
  Forbidden,
  #[error("conflict")]
  Conflict,
  #[error("invalid input: {0}")]
  InvalidInput(InvalidInput),
  #[error(transparent)]
  Other(Box<dyn Error + Send + Sync>),
}

impl CoreError {
  pub fn other<E>(e: E) -> Self
  where
    E: Error + Send + Sync + 'static,
  {
    CoreError::Other(Box::new(e))
  }
}

pub type Result<T> = core::result::Result<T, CoreError>;
