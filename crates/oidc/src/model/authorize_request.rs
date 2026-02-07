use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Authorize request representation (query params)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthorizeRequest {
  pub client_id: String,
  pub response_type: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub response_mode: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub scope: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub redirect_uri: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub state: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub nonce: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub registration: Option<serde_json::Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub code_challenge: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub code_challenge_method: Option<String>,
}
