use alloc::string::String;
use alloc::vec::Vec;
use core::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseOidcEnumError;

impl core::fmt::Display for ParseOidcEnumError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.write_str("invalid OIDC enum value")
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApplicationType {
  Web,
  Native,
}

impl ApplicationType {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Web => "web",
      Self::Native => "native",
    }
  }
}

impl FromStr for ApplicationType {
  type Err = ParseOidcEnumError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "web" => Ok(Self::Web),
      "native" => Ok(Self::Native),
      _ => Err(ParseOidcEnumError),
    }
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SubjectType {
  Public,
  Pairwise,
}

impl SubjectType {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Public => "public",
      Self::Pairwise => "pairwise",
    }
  }
}

impl FromStr for SubjectType {
  type Err = ParseOidcEnumError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "public" => Ok(Self::Public),
      "pairwise" => Ok(Self::Pairwise),
      _ => Err(ParseOidcEnumError),
    }
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseType {
  Code,
  IdToken,
  Token,
  #[serde(other)]
  Unknown,
}

impl ResponseType {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Code => "code",
      Self::IdToken => "id_token",
      Self::Token => "token",
      Self::Unknown => "unknown",
    }
  }
}

impl FromStr for ResponseType {
  type Err = ParseOidcEnumError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "code" => Ok(Self::Code),
      "id_token" => Ok(Self::IdToken),
      "token" => Ok(Self::Token),
      _ => Ok(Self::Unknown),
    }
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GrantType {
  AuthorizationCode,
  RefreshToken,
  ClientCredentials,
  Implicit,
  Password,
  #[serde(rename = "urn:ietf:params:oauth:grant-type:device_code")]
  DeviceCode,
  #[serde(rename = "urn:ietf:params:oauth:grant-type:jwt-bearer")]
  JwtBearer,
  #[serde(other)]
  Unknown,
}

impl GrantType {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::AuthorizationCode => "authorization_code",
      Self::RefreshToken => "refresh_token",
      Self::ClientCredentials => "client_credentials",
      Self::Implicit => "implicit",
      Self::Password => "password",
      Self::DeviceCode => "urn:ietf:params:oauth:grant-type:device_code",
      Self::JwtBearer => "urn:ietf:params:oauth:grant-type:jwt-bearer",
      Self::Unknown => "unknown",
    }
  }
}

impl FromStr for GrantType {
  type Err = ParseOidcEnumError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "authorization_code" => Ok(Self::AuthorizationCode),
      "refresh_token" => Ok(Self::RefreshToken),
      "client_credentials" => Ok(Self::ClientCredentials),
      "implicit" => Ok(Self::Implicit),
      "password" => Ok(Self::Password),
      "urn:ietf:params:oauth:grant-type:device_code" => Ok(Self::DeviceCode),
      "urn:ietf:params:oauth:grant-type:jwt-bearer" => Ok(Self::JwtBearer),
      _ => Ok(Self::Unknown),
    }
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TokenEndpointAuthMethod {
  ClientSecretBasic,
  ClientSecretPost,
  ClientSecretJwt,
  PrivateKeyJwt,
  None,
  #[serde(other)]
  Unknown,
}

impl TokenEndpointAuthMethod {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::ClientSecretBasic => "client_secret_basic",
      Self::ClientSecretPost => "client_secret_post",
      Self::ClientSecretJwt => "client_secret_jwt",
      Self::PrivateKeyJwt => "private_key_jwt",
      Self::None => "none",
      Self::Unknown => "unknown",
    }
  }
}

impl FromStr for TokenEndpointAuthMethod {
  type Err = ParseOidcEnumError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "client_secret_basic" => Ok(Self::ClientSecretBasic),
      "client_secret_post" => Ok(Self::ClientSecretPost),
      "client_secret_jwt" => Ok(Self::ClientSecretJwt),
      "private_key_jwt" => Ok(Self::PrivateKeyJwt),
      "none" => Ok(Self::None),
      _ => Ok(Self::Unknown),
    }
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum JwsAlg {
  Hs256,
  Hs384,
  Hs512,
  Rs256,
  Rs384,
  Rs512,
  Ps256,
  Ps384,
  Ps512,
  Es256,
  Es384,
  Es512,
  #[serde(rename = "EdDSA")]
  EdDsa,
  #[serde(rename = "none")]
  None,
  #[serde(other)]
  Unknown,
}

impl JwsAlg {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Hs256 => "HS256",
      Self::Hs384 => "HS384",
      Self::Hs512 => "HS512",
      Self::Rs256 => "RS256",
      Self::Rs384 => "RS384",
      Self::Rs512 => "RS512",
      Self::Ps256 => "PS256",
      Self::Ps384 => "PS384",
      Self::Ps512 => "PS512",
      Self::Es256 => "ES256",
      Self::Es384 => "ES384",
      Self::Es512 => "ES512",
      Self::EdDsa => "EdDSA",
      Self::None => "none",
      Self::Unknown => "unknown",
    }
  }
}

impl FromStr for JwsAlg {
  type Err = ParseOidcEnumError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "HS256" => Ok(Self::Hs256),
      "HS384" => Ok(Self::Hs384),
      "HS512" => Ok(Self::Hs512),
      "RS256" => Ok(Self::Rs256),
      "RS384" => Ok(Self::Rs384),
      "RS512" => Ok(Self::Rs512),
      "PS256" => Ok(Self::Ps256),
      "PS384" => Ok(Self::Ps384),
      "PS512" => Ok(Self::Ps512),
      "ES256" => Ok(Self::Es256),
      "ES384" => Ok(Self::Es384),
      "ES512" => Ok(Self::Es512),
      "EdDSA" => Ok(Self::EdDsa),
      "none" => Ok(Self::None),
      _ => Ok(Self::Unknown),
    }
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum JweAlg {
  #[serde(rename = "RSA-OAEP")]
  RsaOaep,
  #[serde(rename = "RSA-OAEP-256")]
  RsaOaep256,
  #[serde(rename = "A128KW")]
  A128Kw,
  #[serde(rename = "A256KW")]
  A256Kw,
  #[serde(rename = "dir")]
  Dir,
  #[serde(rename = "ECDH-ES")]
  EcdhEs,
  #[serde(rename = "ECDH-ES+A128KW")]
  EcdhEsA128Kw,
  #[serde(rename = "ECDH-ES+A256KW")]
  EcdhEsA256Kw,
  #[serde(rename = "A128GCMKW")]
  A128GcmKw,
  #[serde(rename = "A256GCMKW")]
  A256GcmKw,
  #[serde(other)]
  Unknown,
}

impl JweAlg {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::RsaOaep => "RSA-OAEP",
      Self::RsaOaep256 => "RSA-OAEP-256",
      Self::A128Kw => "A128KW",
      Self::A256Kw => "A256KW",
      Self::Dir => "dir",
      Self::EcdhEs => "ECDH-ES",
      Self::EcdhEsA128Kw => "ECDH-ES+A128KW",
      Self::EcdhEsA256Kw => "ECDH-ES+A256KW",
      Self::A128GcmKw => "A128GCMKW",
      Self::A256GcmKw => "A256GCMKW",
      Self::Unknown => "unknown",
    }
  }
}

impl FromStr for JweAlg {
  type Err = ParseOidcEnumError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "RSA-OAEP" => Ok(Self::RsaOaep),
      "RSA-OAEP-256" => Ok(Self::RsaOaep256),
      "A128KW" => Ok(Self::A128Kw),
      "A256KW" => Ok(Self::A256Kw),
      "dir" => Ok(Self::Dir),
      "ECDH-ES" => Ok(Self::EcdhEs),
      "ECDH-ES+A128KW" => Ok(Self::EcdhEsA128Kw),
      "ECDH-ES+A256KW" => Ok(Self::EcdhEsA256Kw),
      "A128GCMKW" => Ok(Self::A128GcmKw),
      "A256GCMKW" => Ok(Self::A256GcmKw),
      _ => Ok(Self::Unknown),
    }
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum JweEnc {
  #[serde(rename = "A128GCM")]
  A128Gcm,
  #[serde(rename = "A256GCM")]
  A256Gcm,
  #[serde(rename = "A128CBC-HS256")]
  A128CbcHs256,
  #[serde(rename = "A256CBC-HS512")]
  A256CbcHs512,
  #[serde(other)]
  Unknown,
}

impl JweEnc {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::A128Gcm => "A128GCM",
      Self::A256Gcm => "A256GCM",
      Self::A128CbcHs256 => "A128CBC-HS256",
      Self::A256CbcHs512 => "A256CBC-HS512",
      Self::Unknown => "unknown",
    }
  }
}

impl FromStr for JweEnc {
  type Err = ParseOidcEnumError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "A128GCM" => Ok(Self::A128Gcm),
      "A256GCM" => Ok(Self::A256Gcm),
      "A128CBC-HS256" => Ok(Self::A128CbcHs256),
      "A256CBC-HS512" => Ok(Self::A256CbcHs512),
      _ => Ok(Self::Unknown),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct OidcClientMeta {
  pub client_id: Option<String>,
  pub client_secret: Option<String>,
  #[serde(default)]
  pub redirect_uris: Vec<String>,
  #[serde(default)]
  pub response_types: Vec<ResponseType>,
  #[serde(default)]
  pub grant_types: Vec<GrantType>,
  pub scope: Option<String>,
  pub require_pkce: Option<bool>,
  pub application_type: Option<ApplicationType>,
  pub contacts: Option<Vec<String>>,
  pub client_name: Option<String>,
  pub client_uri: Option<String>,
  pub logo_uri: Option<String>,
  pub policy_uri: Option<String>,
  pub tos_uri: Option<String>,
  pub jwks_uri: Option<String>,
  pub jwks: Option<JsonValue>,
  pub sector_identifier_uri: Option<String>,
  pub subject_type: Option<SubjectType>,
  pub id_token_signed_response_alg: Option<JwsAlg>,
  pub id_token_encrypted_response_alg: Option<JweAlg>,
  pub id_token_encrypted_response_enc: Option<JweEnc>,
  pub userinfo_signed_response_alg: Option<JwsAlg>,
  pub userinfo_encrypted_response_alg: Option<JweAlg>,
  pub userinfo_encrypted_response_enc: Option<JweEnc>,
  pub request_object_signing_alg: Option<JwsAlg>,
  pub request_object_encryption_alg: Option<JweAlg>,
  pub request_object_encryption_enc: Option<JweEnc>,
  pub token_endpoint_auth_method: Option<TokenEndpointAuthMethod>,
  pub token_endpoint_auth_signing_alg: Option<JwsAlg>,
  pub default_max_age: Option<u64>,
  pub require_auth_time: Option<bool>,
  pub default_acr_values: Option<Vec<String>>,
  pub initiate_login_uri: Option<String>,
  #[serde(default)]
  pub request_uris: Vec<String>,
  #[serde(default)]
  pub post_logout_redirect_uris: Vec<String>,
  pub frontchannel_logout_uri: Option<String>,
  pub frontchannel_logout_session_required: Option<bool>,
  pub backchannel_logout_uri: Option<String>,
  pub backchannel_logout_session_required: Option<bool>,
}
