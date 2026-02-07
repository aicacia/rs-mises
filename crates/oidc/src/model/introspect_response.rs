use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Introspection response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntrospectResponse {
  pub active: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub scope: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub client_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub username: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub token_type: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exp: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub iat: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub nbf: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub sub: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub aud: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub iss: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub jti: Option<String>,
}
