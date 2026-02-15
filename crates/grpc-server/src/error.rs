use mises_core::CoreError;
use mises_graph::GraphError;
use tonic::Status;

pub trait ToStatus {
  fn to_status(&self) -> Status;
}

impl ToStatus for CoreError {
  fn to_status(&self) -> Status {
    match self {
      CoreError::NotFound => Status::not_found(self.to_string()),
      CoreError::InvalidInput(_) => Status::invalid_argument(self.to_string()),
      CoreError::Conflict => Status::already_exists(self.to_string()),
      CoreError::Graph(graph_err) => Status::internal(format!("graph error: {}", graph_err)),
      CoreError::Key(key_err) => Status::internal(format!("key error: {}", key_err)),
      CoreError::Serde(serde_err) => {
        Status::internal(format!("serialization error: {}", serde_err))
      }
      CoreError::Other(_) => Status::internal(self.to_string()),
    }
  }
}

impl ToStatus for GraphError {
  fn to_status(&self) -> Status {
    match self {
      Self::Conflict => Status::already_exists("conflict".to_string()),
      Self::NotFound => Status::not_found("not found".to_string()),
      Self::SerializationError(e) => Status::internal(format!("serialization error: {}", e)),
      Self::Other(msg) => Status::internal(msg.to_string()),
    }
  }
}
