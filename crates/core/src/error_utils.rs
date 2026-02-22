use alloc::string::{String, ToString};

use mises_graph::GraphError;

use crate::CoreError;

/// Check if a `CoreError` represents a "not found" condition.
pub fn is_not_found_error(err: &CoreError) -> bool {
  matches!(
    err,
    CoreError::NotFound | CoreError::Graph(GraphError::NotFound)
  )
}

/// Check if a `CoreError` represents a "conflict" condition.
pub fn is_conflict_error(err: &CoreError) -> bool {
  matches!(
    err,
    CoreError::Conflict | CoreError::Graph(GraphError::Conflict)
  )
}

/// Get a standardized error code string for a `CoreError`.
///
/// Returns a machine-readable error code suitable for API responses.
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

/// Check if a `GraphError` represents a "not found" condition.
pub fn graph_is_not_found(err: &GraphError) -> bool {
  matches!(err, GraphError::NotFound)
}

/// Check if a `GraphError` represents a "conflict" condition.
pub fn graph_is_conflict(err: &GraphError) -> bool {
  matches!(err, GraphError::Conflict)
}
