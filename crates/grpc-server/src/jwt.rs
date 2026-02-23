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

pub struct TokenBuilder<'a> {
  key_node: &'a Node<Uuid, NodeMeta>,
  sub: Option<String>,
  issuer: Option<String>,
  audience: Option<String>,
  expires_in_seconds: Option<i64>,
  scope: Option<String>,
  acting_for: Option<String>,
  nonce: Option<String>,
  token_type: Option<String>,
}

impl<'a> TokenBuilder<'a> {
  pub fn new(key_node: &'a Node<Uuid, NodeMeta>) -> Self {
    Self {
      key_node,
      sub: None,
      issuer: None,
      audience: None,
      expires_in_seconds: None,
      scope: None,
      acting_for: None,
      nonce: None,
      token_type: None,
    }
  }

  pub fn sub(mut self, sub: impl Into<String>) -> Self {
    self.sub = Some(sub.into());
    self
  }

  pub fn issuer(mut self, issuer: impl Into<String>) -> Self {
    self.issuer = Some(issuer.into());
    self
  }

  pub fn audience(mut self, audience: impl Into<String>) -> Self {
    self.audience = Some(audience.into());
    self
  }

  pub fn expires_in(mut self, seconds: i64) -> Self {
    self.expires_in_seconds = Some(seconds);
    self
  }

  pub fn scope(mut self, scope: impl Into<String>) -> Self {
    self.scope = Some(scope.into());
    self
  }

  pub fn acting_for(mut self, acting_for: impl Into<String>) -> Self {
    self.acting_for = Some(acting_for.into());
    self
  }

  pub fn nonce(mut self, nonce: impl Into<String>) -> Self {
    self.nonce = Some(nonce.into());
    self
  }

  pub fn token_type(mut self, token_type: impl Into<String>) -> Self {
    self.token_type = Some(token_type.into());
    self
  }

  pub fn build(self) -> Result<String, Status> {
    let key = match &self.key_node.metadata {
      NodeMeta::Key(key_meta) => key_meta,
      _ => return Err(Status::internal("expected key node")),
    };

    let encoding_key = key.jwt_encoding_key().map_err(|e| {
      log::error!("failed to derive jwt encoding key: {}", e);
      Status::internal("invalid secret key format")
    })?;

    let now = Utc::now();
    let exp = self.expires_in_seconds.map(|s| now + Duration::seconds(s));

    let claims = Claims {
      sub: self.sub.unwrap_or_default(),
      iss: self.issuer,
      iat: Some(now.timestamp()),
      exp: exp.map(|dt| dt.timestamp()),
      jti: Some(Uuid::new_v4().to_string()),
      aud: self.audience,
      scope: self.scope.or(self.nonce),
      acting_for: self.acting_for,
    };

    let token_type = self.token_type.unwrap_or_else(|| "access".to_string());

    encode(
      &Header {
        alg: Algorithm::EdDSA,
        kid: Some(self.key_node.id.to_string()),
        ..Default::default()
      },
      &claims,
      &encoding_key,
    )
    .map_err(|e| Status::internal(format!("failed to generate {} token: {}", token_type, e)))
  }
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
