use alloc::string::{String, ToString};

use mises_graph::GraphError;

use crate::CoreError;

pub fn is_not_found_error(err: &CoreError) -> bool {
  matches!(
    err,
    CoreError::NotFound | CoreError::Graph(GraphError::NotFound)
  )
}

pub fn is_conflict_error(err: &CoreError) -> bool {
  matches!(
    err,
    CoreError::Conflict | CoreError::Graph(GraphError::Conflict)
  )
}

pub fn error_code(err: &CoreError) -> String {
  if is_not_found_error(err) {
    "NOT_FOUND".to_string()
  } else if is_conflict_error(err) {
    "CONFLICT".to_string()
  } else {
    match err {
      CoreError::Graph(_) => "GRAPH_ERROR".to_string(),
      CoreError::Key(_) => "KEY_ERROR".to_string(),
      CoreError::Serde(_) => "SERIALIZATION_ERROR".to_string(),
      CoreError::InvalidInput(_) => "INVALID_INPUT".to_string(),
      CoreError::Other(_) => "INTERNAL_ERROR".to_string(),
      _ => "UNKNOWN_ERROR".to_string(),
    }
  }
}

pub fn graph_is_not_found(err: &GraphError) -> bool {
  matches!(err, GraphError::NotFound)
}

pub fn graph_is_conflict(err: &GraphError) -> bool {
  matches!(err, GraphError::Conflict)
}
