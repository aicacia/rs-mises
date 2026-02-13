use base64::Engine;
use serde::Deserialize;
use tonic::Status;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Claims {
  pub sub: String,
  pub exp: Option<i64>,
  pub iat: Option<i64>,
  pub iss: Option<String>,
  pub aud: Option<String>,
}

pub fn extract_auth_header(auth_header: &str) -> Result<&str, Status> {
  let parts: Vec<&str> = auth_header.split_whitespace().collect();
  if parts.len() != 2 {
    return Err(Status::unauthenticated(
      "invalid authorization header format",
    ));
  }

  if !parts[0].eq_ignore_ascii_case("bearer") {
    return Err(Status::unauthenticated(
      "authorization header must use Bearer scheme",
    ));
  }

  Ok(parts[1])
}

pub fn parse_jwt_claims(token: &str) -> Result<Claims, Status> {
  let parts: Vec<&str> = token.split('.').collect();
  if parts.len() != 3 {
    return Err(Status::unauthenticated("invalid JWT format"));
  }

  let claims_part = parts[1];
  let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
    .decode(claims_part)
    .map_err(|_| Status::unauthenticated("failed to decode claims"))?;

  serde_json::from_slice(&decoded).map_err(|_| Status::unauthenticated("invalid JWT claims JSON"))
}

pub fn extract_and_parse_jwt_claims(auth_header: &str) -> Result<Claims, Status> {
  let token = extract_auth_header(auth_header)?;
  parse_jwt_claims(token)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_auth_header_valid() {
    let result = extract_auth_header("Bearer token123");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "token123");
  }

  #[test]
  fn test_extract_auth_header_invalid_format() {
    let result = extract_auth_header("Bearer");
    assert!(result.is_err());
  }

  #[test]
  fn test_extract_auth_header_invalid_scheme() {
    let result = extract_auth_header("Basic token123");
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_jwt_claims_invalid_format() {
    let result = parse_jwt_claims("invalid");
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_jwt_claims_invalid_base64() {
    let result = parse_jwt_claims("header.!!!invalid!!!.signature");
    assert!(result.is_err());
  }
}
