use alloc::string::String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OidcError {
  InvalidRequest(Option<String>),
  InvalidClient(Option<String>),
  InvalidGrant(Option<String>),
  UnauthorizedClient(Option<String>),
  UnsupportedGrantType(Option<String>),
  InvalidScope(Option<String>),
  NotFound(Option<String>),
  ServerError(Option<String>),
  Other(Option<String>),
}

impl OidcError {
  pub fn message(&self) -> Option<&str> {
    match self {
      Self::InvalidRequest(m) => m.as_deref(),
      Self::InvalidClient(m) => m.as_deref(),
      Self::InvalidGrant(m) => m.as_deref(),
      Self::UnauthorizedClient(m) => m.as_deref(),
      Self::UnsupportedGrantType(m) => m.as_deref(),
      Self::InvalidScope(m) => m.as_deref(),
      Self::NotFound(m) => m.as_deref(),
      Self::ServerError(m) => m.as_deref(),
      Self::Other(m) => m.as_deref(),
    }
  }
}

#[cfg(feature = "std")]
impl std::fmt::Display for OidcError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::InvalidRequest(m) => write!(f, "invalid_request: {}", m.as_deref().unwrap_or("")),
      Self::InvalidClient(m) => write!(f, "invalid_client: {}", m.as_deref().unwrap_or("")),
      Self::InvalidGrant(m) => write!(f, "invalid_grant: {}", m.as_deref().unwrap_or("")),
      Self::UnauthorizedClient(m) => {
        write!(f, "unauthorized_client: {}", m.as_deref().unwrap_or(""))
      }
      Self::UnsupportedGrantType(m) => {
        write!(f, "unsupported_grant_type: {}", m.as_deref().unwrap_or(""))
      }
      Self::InvalidScope(m) => write!(f, "invalid_scope: {}", m.as_deref().unwrap_or("")),
      Self::NotFound(m) => write!(f, "not_found: {}", m.as_deref().unwrap_or("")),
      Self::ServerError(m) => write!(f, "server_error: {}", m.as_deref().unwrap_or("")),
      Self::Other(m) => write!(f, "other: {}", m.as_deref().unwrap_or("")),
    }
  }
}

#[cfg(feature = "std")]
impl std::error::Error for OidcError {}
