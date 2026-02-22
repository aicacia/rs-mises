use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, Header, Validation, decode, decode_header, encode};
use serde::{Deserialize, Serialize};
use tonic::{Request, Status};
use uuid::Uuid;

use mises_core::{
  CoreError, model::node::NodeMeta, service::identity::IdentityService, traits::Repository,
};
use mises_graph::Node;

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

fn generate_token(
  key_node: &Node<Uuid, NodeMeta>,
  sub: &str,
  issuer: &str,
  audience: &str,
  expires_in_seconds: i64,
  scope: Option<String>,
  acting_for: Option<String>,
  token_type: &str,
) -> Result<String, Status> {
  let key = match &key_node.metadata {
    NodeMeta::Key(key_meta) => key_meta,
    _ => return Err(Status::internal("expected key node")),
  };

  let encoding_key = key.jwt_encoding_key().map_err(|e| {
    log::error!("failed to derive jwt encoding key: {}", e);
    Status::internal("invalid secret key format")
  })?;

  let now = Utc::now();
  let exp = now + Duration::seconds(expires_in_seconds);

  let claims = Claims {
    sub: sub.to_string(),
    iss: Some(issuer.to_string()),
    iat: Some(now.timestamp()),
    exp: Some(exp.timestamp()),
    jti: Some(Uuid::new_v4().to_string()),
    aud: Some(audience.to_string()),
    scope,
    acting_for,
  };

  encode(
    &Header {
      alg: Algorithm::EdDSA,
      kid: Some(key_node.id.to_string()),
      ..Default::default()
    },
    &claims,
    &encoding_key,
  )
  .map_err(|e| Status::internal(format!("failed to generate {} token: {}", token_type, e)))
}

pub fn generate_access_token(
  key_node: &Node<Uuid, NodeMeta>,
  sub: &str,
  issuer: &str,
  audience: &str,
  scope: Option<&str>,
  acting_for: Option<&str>,
  expires_in_seconds: i64,
) -> Result<String, Status> {
  generate_token(
    key_node,
    sub,
    issuer,
    audience,
    expires_in_seconds,
    scope.map(ToString::to_string),
    acting_for.map(ToString::to_string),
    "access",
  )
}

pub fn generate_refresh_token(
  key_node: &Node<Uuid, NodeMeta>,
  sub: &str,
  issuer: &str,
  audience: &str,
  scope: Option<&str>,
  acting_for: Option<&str>,
  expires_in_seconds: i64,
) -> Result<String, Status> {
  generate_token(
    key_node,
    sub,
    issuer,
    audience,
    expires_in_seconds,
    scope.map(ToString::to_string),
    acting_for.map(ToString::to_string),
    "refresh",
  )
}

pub fn generate_id_token(
  key_node: &Node<Uuid, NodeMeta>,
  sub: &str,
  issuer: &str,
  audience: &str,
  nonce: Option<&str>,
  expires_in_seconds: i64,
) -> Result<String, Status> {
  generate_token(
    key_node,
    sub,
    issuer,
    audience,
    expires_in_seconds,
    nonce.map(|n| format!("nonce:{}", n)),
    None,
    "id",
  )
}

pub async fn extract_optional_claims<T, R>(
  request: &Request<T>,
  identity_service: IdentityService<R>,
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

  let kid = Uuid::parse_str(&kid_string).map_err(|e| {
    log::error!("invalid key id in token header: {}", e);
    Status::unauthenticated("invalid key id in token header")
  })?;

  let key = identity_service
    .get_key_by_id(kid)
    .await
    .map_err(|e| match e {
      CoreError::NotFound => {
        log::error!("key node not found for kid: {}", kid);
        Status::unauthenticated("invalid key id in token header")
      }
      e => {
        log::error!("failed to retrieve key node: {}", e);
        Status::internal("failed to retrieve key node")
      }
    })?;

  let decoding_key = key.jwt_decoding_key().map_err(|e| {
    log::error!("failed to derive jwt decoding key: {}", e);
    Status::unauthenticated("invalid public key format")
  })?;

  let service = identity_service
    .find_service_by_name("mises")
    .await
    .map_err(|e| {
      log::error!(
        "failed to find mises service for audience validation: {}",
        e
      );
      Status::internal("failed to resolve audience")
    })?
    .ok_or_else(|| Status::unauthenticated("mises service not found"))?;
  let service_id = service.id.to_string();

  let mut validation = Validation::new(Algorithm::EdDSA);
  validation.validate_exp = true;
  validation.set_audience(&[service_id.as_str()]);

  let token_data = decode::<Claims>(token, &decoding_key, &validation).map_err(|e| {
    log::error!("failed to decode token: {}", e);
    Status::unauthenticated("invalid token signature")
  })?;

  Ok(Some(token_data.claims))
}
