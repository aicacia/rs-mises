use alloc::string::String;
use serde::{Deserialize, Serialize};

/// OIDC UserInfo
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserInfo {
  pub sub: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub given_name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub family_name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub preferred_username: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub email: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub email_verified: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub picture: Option<String>,
}
