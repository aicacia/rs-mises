use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Device Authorization Response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceAuthorizeResponse {
  pub device_code: String,
  pub user_code: String,
  pub verification_uri: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub verification_uri_complete: Option<String>,
  pub expires_in: u64,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub interval: Option<u64>,
}
