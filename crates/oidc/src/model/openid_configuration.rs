use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

/// OpenID Provider Configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenIdConfiguration {
  pub issuer: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub jwks_uri: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub authorization_endpoint: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub token_endpoint: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub userinfo_endpoint: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub end_session_endpoint: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub response_types_supported: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub subject_types_supported: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id_token_signing_alg_values_supported: Option<Vec<String>>,
}
