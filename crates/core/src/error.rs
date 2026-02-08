use alloc::{boxed::Box, string::String};

use core::{
  error::Error,
  fmt::{self, Display, Formatter},
};

use mises_graph::GraphError;
use mises_key::KeyError;
use serde_json::Error as SerdeError;

#[derive(Debug)]
pub enum InvalidInput {
  SerializationFailed(String),
  Other(String),
}

impl Display for InvalidInput {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      InvalidInput::SerializationFailed(s) => write!(f, "failed to serialize {}", s),
      InvalidInput::Other(s) => write!(f, "{}", s),
    }
  }
}

impl core::error::Error for InvalidInput {}

#[derive(Debug)]
pub enum CoreError {
  Graph(GraphError),
  Key(KeyError),
  Serde(SerdeError),
  NotFound,
  Conflict,
  InvalidInput(InvalidInput),
  Other(Box<dyn Error + Send + Sync>),
}

impl From<GraphError> for CoreError {
  fn from(e: GraphError) -> Self {
    CoreError::Graph(e)
  }
}

impl From<KeyError> for CoreError {
  fn from(e: KeyError) -> Self {
    CoreError::Key(e)
  }
}

impl From<SerdeError> for CoreError {
  fn from(e: SerdeError) -> Self {
    CoreError::Serde(e)
  }
}

impl CoreError {
  pub fn other<E>(e: E) -> Self
  where
    E: Error + Send + Sync + 'static,
  {
    CoreError::Other(Box::new(e))
  }
}

impl Display for CoreError {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      CoreError::Graph(e) => write!(f, "graph error: {}", e),
      CoreError::Key(e) => write!(f, "key error: {}", e),
      CoreError::Serde(e) => write!(f, "serde error: {}", e),
      CoreError::NotFound => write!(f, "not found"),
      CoreError::Conflict => write!(f, "conflict"),
      CoreError::InvalidInput(s) => write!(f, "invalid input: {}", s),
      CoreError::Other(e) => write!(f, "other error: {}", e),
    }
  }
}

impl Error for CoreError {}

pub type Result<T> = core::result::Result<T, CoreError>;
