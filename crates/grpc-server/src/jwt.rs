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

pub fn generate_access_token(
  sub: &str,
  issuer: &str,
  audience: &str,
  scope: Option<&str>,
  acting_for: Option<&str>,
  expires_in_seconds: i64,
  secret: &EncodingKey,
) -> Result<String, Status> {
  let now = Utc::now();
  let exp = now + Duration::seconds(expires_in_seconds);

  let claims = Claims {
    sub: sub.to_string(),
    iss: Some(issuer.to_string()),
    iat: Some(now.timestamp()),
    exp: Some(exp.timestamp()),
    jti: Some(Uuid::new_v4().to_string()),
    aud: Some(audience.to_string()),
    scope: scope.map(ToString::to_string),
    acting_for: acting_for.map(ToString::to_string),
  };

  encode(&Header::default(), &claims, secret)
    .map_err(|e| Status::internal(format!("failed to generate access token: {}", e)))
}

pub fn generate_refresh_token(
  sub: &str,
  issuer: &str,
  audience: &str,
  scope: Option<&str>,
  acting_for: Option<&str>,
  expires_in_seconds: i64,
  secret: &EncodingKey,
) -> Result<String, Status> {
  let now = Utc::now();
  let exp = now + Duration::seconds(expires_in_seconds);

  let claims = Claims {
    sub: sub.to_string(),
    iss: Some(issuer.to_string()),
    iat: Some(now.timestamp()),
    exp: Some(exp.timestamp()),
    jti: Some(Uuid::new_v4().to_string()),
    aud: Some(audience.to_string()),
    scope: scope.map(ToString::to_string),
    acting_for: acting_for.map(ToString::to_string),
  };

  encode(&Header::default(), &claims, secret)
    .map_err(|e| Status::internal(format!("failed to generate refresh token: {}", e)))
}

pub fn generate_id_token(
  sub: &str,
  issuer: &str,
  audience: &str,
  nonce: Option<&str>,
  expires_in_seconds: i64,
  secret: &EncodingKey,
) -> Result<String, Status> {
  let now = Utc::now();
  let exp = now + Duration::seconds(expires_in_seconds);

  let mut claims = Claims {
    sub: sub.to_string(),
    iss: Some(issuer.to_string()),
    iat: Some(now.timestamp()),
    exp: Some(exp.timestamp()),
    jti: Some(Uuid::new_v4().to_string()),
    aud: Some(audience.to_string()),
    scope: None,
    acting_for: None,
  };

  if let Some(nonce_val) = nonce {
    claims.scope = Some(format!("nonce:{}", nonce_val));
  }

  encode(&Header::default(), &claims, secret)
    .map_err(|e| Status::internal(format!("failed to generate id token: {}", e)))
}
