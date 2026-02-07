use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

/// Minimal subset of standard ID Token claims used in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdTokenClaims {
  pub iss: String,
  pub sub: String,
  #[serde(default)]
  pub aud: Vec<String>,
  pub exp: u64,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub iat: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub jti: Option<String>,
  /// optional delegation claim per design: `acting_for`
  #[serde(rename = "acting_for", skip_serializing_if = "Option::is_none")]
  pub acting_for: Option<String>,
  /// optional scope claim
  #[serde(skip_serializing_if = "Option::is_none")]
  pub scope: Option<String>,
}
