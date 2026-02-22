use tonic::Status;
use uuid::Uuid;

pub trait ResultExt<T> {
  fn or_invalid_argument(self, message: &str) -> Result<T, Status>;
  fn or_not_found(self, message: &str) -> Result<T, Status>;
  fn or_internal(self, message: &str) -> Result<T, Status>;
  fn or_unauthenticated(self, message: &str) -> Result<T, Status>;
}

impl<T, E> ResultExt<T> for Result<T, E> {
  fn or_invalid_argument(self, message: &str) -> Result<T, Status> {
    self.map_err(|_| Status::invalid_argument(message))
  }

  fn or_not_found(self, message: &str) -> Result<T, Status> {
    self.map_err(|_| Status::not_found(message))
  }

  fn or_internal(self, message: &str) -> Result<T, Status> {
    self.map_err(|_| Status::internal(message))
  }

  fn or_unauthenticated(self, message: &str) -> Result<T, Status> {
    self.map_err(|_| Status::unauthenticated(message))
  }
}

pub trait OptionExt<T> {
  fn or_invalid_argument(self, message: &str) -> Result<T, Status>;
  fn or_not_found(self, message: &str) -> Result<T, Status>;
  fn or_internal(self, message: &str) -> Result<T, Status>;
  fn or_unauthenticated(self, message: &str) -> Result<T, Status>;
}

impl<T> OptionExt<T> for Option<T> {
  fn or_invalid_argument(self, message: &str) -> Result<T, Status> {
    self.ok_or_else(|| Status::invalid_argument(message))
  }

  fn or_not_found(self, message: &str) -> Result<T, Status> {
    self.ok_or_else(|| Status::not_found(message))
  }

  fn or_internal(self, message: &str) -> Result<T, Status> {
    self.ok_or_else(|| Status::internal(message))
  }

  fn or_unauthenticated(self, message: &str) -> Result<T, Status> {
    self.ok_or_else(|| Status::unauthenticated(message))
  }
}

pub fn parse_uuid(s: &str) -> Result<Uuid, Status> {
  Uuid::parse_str(s).or_invalid_argument("invalid UUID format")
}
