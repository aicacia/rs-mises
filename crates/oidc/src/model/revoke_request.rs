use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Token Revocation Request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevokeRequest {
  pub token: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub token_type_hint: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub client_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub client_secret: Option<String>,
}
