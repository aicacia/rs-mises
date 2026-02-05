use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

/// JSON Web Key (minimal fields used by the client)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Jwk {
  pub kty: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub r#use: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub alg: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub kid: Option<String>,

  // RSA
  #[serde(skip_serializing_if = "Option::is_none")]
  pub n: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub e: Option<String>,

  // EC
  #[serde(skip_serializing_if = "Option::is_none")]
  pub crv: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub x: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub y: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Jwks {
  pub keys: Vec<Jwk>,
}

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
