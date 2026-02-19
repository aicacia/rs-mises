use chrono::{Duration, Utc};
use jsonwebtoken::{
  Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode,
};
use serde::{Deserialize, Serialize};
use tonic::{Request, Status};
use uuid::Uuid;

use mises_core::{model::node::NodeMeta, traits::Repository};

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
  kid: &str,
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

  encode(
    &Header {
      alg: Algorithm::EdDSA,
      kid: Some(kid.to_string()),
      ..Default::default()
    },
    &claims,
    secret,
  )
  .map_err(|e| Status::internal(format!("failed to generate access token: {}", e)))
}

pub fn generate_refresh_token(
  kid: &str,
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

  encode(
    &Header {
      alg: Algorithm::EdDSA,
      kid: Some(kid.to_string()),
      ..Default::default()
    },
    &claims,
    secret,
  )
  .map_err(|e| Status::internal(format!("failed to generate refresh token: {}", e)))
}

pub fn generate_id_token(
  kid: &str,
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

  encode(
    &Header {
      alg: Algorithm::EdDSA,
      kid: Some(kid.to_string()),
      ..Default::default()
    },
    &claims,
    secret,
  )
  .map_err(|e| Status::internal(format!("failed to generate id token: {}", e)))
}

pub async fn extract_optional_claims<T, R>(
  request: &Request<T>,
  key_repository: &R,
) -> Result<Option<Claims>, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  let Some(auth_header) = request
    .metadata()
    .get("authorization")
    .and_then(|v| v.to_str().ok())
  else {
    return Ok(None);
  };

  let auth_header = auth_header.trim();
  if auth_header.is_empty() {
    return Ok(None);
  }

  let token = auth_header
    .strip_prefix("Bearer ")
    .or_else(|| auth_header.strip_prefix("bearer "))
    .ok_or_else(|| Status::unauthenticated("invalid bearer token"))?;

  let header = decode_header(token).map_err(|_| Status::unauthenticated("invalid bearer token"))?;

  let Some(kid_string) = header.kid else {
    return Err(Status::unauthenticated("missing key id in token header"));
  };

  let kid = Uuid::try_from(kid_string).map_err(|e| {
    log::error!("invalid key id in token header: {}", e);
    Status::unauthenticated("invalid key id in token header")
  })?;

  let Some(key_node) = key_repository
    .get_node_by_id(kid)
    .await
    .map_err(|_| Status::unauthenticated("invalid key id"))?
  else {
    return Err(Status::unauthenticated("key not found"));
  };

  let public_key = match key_node.metadata {
    NodeMeta::Key(key) => match key.decode_public_key_bytes() {
      Ok(bytes) => bytes,
      Err(e) => {
        log::error!("invalid public key format in key node: {}", e);
        return Err(Status::unauthenticated(
          "invalid public key format in key node",
        ));
      }
    },
    _ => {
      log::error!("key node does not contain a public key");
      return Err(Status::unauthenticated("invalid key type"));
    }
  };

  let decoding_key = DecodingKey::from_ed_der(&public_key);

  let token_data = decode::<Claims>(token, &decoding_key, &Validation::new(Algorithm::EdDSA))
    .map_err(|_| Status::unauthenticated("invalid token signature"))?;

  Ok(Some(token_data.claims))
}
