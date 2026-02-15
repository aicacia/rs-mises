use base64::Engine;
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use tonic::Status;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
  pub sub: String,
  pub exp: Option<i64>,
  pub iat: Option<i64>,
  pub iss: Option<String>,
  pub aud: Option<String>,
  pub jti: Option<String>,
  pub scope: Option<String>,
  pub acting_for: Option<String>,
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

pub fn generate_access_token(
  sub: &str,
  issuer: &str,
  scope: Option<&str>,
  expires_in_seconds: i64,
  secret: &[u8],
) -> Result<String, Status> {
  let now = Utc::now();
  let exp = now + Duration::seconds(expires_in_seconds);

  let claims = Claims {
    sub: sub.to_string(),
    iss: Some(issuer.to_string()),
    iat: Some(now.timestamp()),
    exp: Some(exp.timestamp()),
    jti: Some(Uuid::new_v4().to_string()),
    aud: None,
    scope: scope.map(ToString::to_string),
    acting_for: None,
  };

  encode(
    &Header::default(),
    &claims,
    &EncodingKey::from_secret(secret),
  )
  .map_err(|e| Status::internal(format!("failed to generate access token: {}", e)))
}

pub fn generate_refresh_token(
  sub: &str,
  issuer: &str,
  scope: Option<&str>,
  expires_in_seconds: i64,
  secret: &[u8],
) -> Result<String, Status> {
  let now = Utc::now();
  let exp = now + Duration::seconds(expires_in_seconds);

  let claims = Claims {
    sub: sub.to_string(),
    iss: Some(issuer.to_string()),
    iat: Some(now.timestamp()),
    exp: Some(exp.timestamp()),
    jti: Some(Uuid::new_v4().to_string()),
    aud: None,
    scope: scope.map(ToString::to_string),
    acting_for: None,
  };

  encode(
    &Header::default(),
    &claims,
    &EncodingKey::from_secret(secret),
  )
  .map_err(|e| Status::internal(format!("failed to generate refresh token: {}", e)))
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
