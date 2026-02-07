use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Client model returned by server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Client {
  pub id: Uuid,
  pub client_id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub client_secret: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub redirect_uris: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub grant_types: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub response_types: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub scope: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub token_endpoint_auth_method: Option<String>,
}
