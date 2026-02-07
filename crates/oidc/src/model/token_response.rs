use alloc::string::String;
use serde::{Deserialize, Serialize};

/// OAuth2 / OIDC Token Response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenResponse {
  pub access_token: String,
  pub token_type: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub expires_in: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub refresh_token: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id_token: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub scope: Option<String>,
}
