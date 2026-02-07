use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

/// Client registration request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientRegisterRequest {
  pub client_id: Option<String>,
  pub client_secret: Option<String>,
  pub name: Option<String>,
  pub redirect_uris: Option<Vec<String>>,
  pub grant_types: Option<Vec<String>>,
  pub response_types: Option<Vec<String>>,
  pub scope: Option<String>,
  pub token_endpoint_auth_method: Option<String>,
}
