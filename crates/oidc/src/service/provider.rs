use alloc::string::String;

use crate::OidcError;
use crate::model::*;

use mises_core::{service::graph::KeyVault, traits::Repository};

#[derive(Clone)]
pub struct OidcProvider<R, V>
where
  R: Repository,
  V: KeyVault,
{
  pub repo: R,
  pub key_vault: V,
  pub issuer: Option<String>,
}

impl<R, V> OidcProvider<R, V>
where
  R: Repository,
  V: KeyVault,
{
  pub fn new(repo: R, key_vault: V, issuer: Option<String>) -> Self {
    Self {
      repo,
      key_vault,
      issuer,
    }
  }

  /// Return the provider OpenID configuration
  pub async fn openid_configuration(&self) -> Result<OpenIdConfiguration, OidcError> {
    unimplemented!()
  }

  /// Return the provider JWKS
  pub async fn jwks(&self) -> Result<Jwks, OidcError> {
    unimplemented!()
  }

  /// Process a token request and return a token response or error
  pub async fn token(
    &self,
    _req: crate::service::TokenRequest,
  ) -> Result<TokenResponse, OidcError> {
    unimplemented!()
  }

  /// Introspect a token
  pub async fn introspect(&self, _token: String) -> Result<IntrospectResponse, OidcError> {
    unimplemented!()
  }

  /// Return userinfo for an access token
  pub async fn userinfo(&self, _access_token: String) -> Result<UserInfo, OidcError> {
    unimplemented!()
  }

  /// Register a client
  pub async fn register_client(&self, _req: ClientRegisterRequest) -> Result<Client, OidcError> {
    unimplemented!()
  }

  /// Revoke a token
  pub async fn revoke(&self, _req: RevokeRequest) -> Result<(), OidcError> {
    unimplemented!()
  }

  /// Device authorization flow initiation
  pub async fn device_authorize(
    &self,
    _client_id: String,
    _scope: Option<String>,
  ) -> Result<DeviceAuthorizeResponse, OidcError> {
    unimplemented!()
  }
}
