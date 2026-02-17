use std::collections::HashSet;

use tonic::Status;
use url::Url;

use mises_core::{
  model::{identity::IdentityMeta, node::NodeMeta},
  service::identity::IdentityService,
  traits::Repository,
};

use crate::oidc_service::constants;

pub async fn get_open_id_configuration<R>(
  repo: &R,
  device_id: &str,
  issuer: String,
  public_uri: &Option<Url>,
) -> Result<mises_proto::OpenIdConfiguration, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  let identity_service = IdentityService::new(repo.clone(), device_id.to_string());
  let applications = identity_service
    .list_applications()
    .await
    .map_err(|e| Status::internal(format!("list_applications error: {}", e)))?;

  let mut supported_response_types: HashSet<String> = Default::default();
  let mut supported_grant_types: HashSet<String> = Default::default();

  for app in applications {
    if let NodeMeta::Identity(identity) = &app.metadata
      && let IdentityMeta::Application { oidc, .. } = identity.as_ref()
      && let Some(oidc_meta) = oidc.as_ref()
    {
      for rt in &oidc_meta.response_types {
        supported_response_types.insert(rt.as_str().to_string());
      }
      for gt in &oidc_meta.grant_types {
        supported_grant_types.insert(gt.as_str().to_string());
      }
    }
  }

  let (
    jwks_uri,
    authorization_endpoint,
    token_endpoint,
    userinfo_endpoint,
    end_session_endpoint,
    revocation_endpoint,
    introspection_endpoint,
    registration_endpoint,
    device_authorization_endpoint,
    pushed_authorization_request_endpoint,
    check_session_iframe,
  ) = if let Some(public_uri) = public_uri {
    (
      public_uri.join("/jwks.json").map(|u| u.to_string()).ok(),
      public_uri.join("/authorize").map(|u| u.to_string()).ok(),
      public_uri.join("/token").map(|u| u.to_string()).ok(),
      public_uri.join("/user-info").map(|u| u.to_string()).ok(),
      public_uri.join("/end_session").map(|u| u.to_string()).ok(),
      public_uri.join("/revoke").map(|u| u.to_string()).ok(),
      public_uri.join("/introspect").map(|u| u.to_string()).ok(),
      public_uri.join("/register").map(|u| u.to_string()).ok(),
      public_uri
        .join("/device_authorize")
        .map(|u| u.to_string())
        .ok(),
      public_uri
        .join("/pushed_authorize")
        .map(|u| u.to_string())
        .ok(),
      public_uri
        .join("/check_session")
        .map(|u| u.to_string())
        .ok(),
    )
  } else {
    (
      None, None, None, None, None, None, None, None, None, None, None,
    )
  };

  let response_types_supported: Vec<String> = if supported_response_types.is_empty() {
    constants::RESPONSE_TYPES_SUPPORTED
      .iter()
      .map(|s| s.to_string())
      .collect()
  } else {
    supported_response_types.into_iter().collect()
  };

  let response_modes_supported = constants::RESPONSE_MODES
    .iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

  let grant_types_supported: Vec<String> = if supported_grant_types.is_empty() {
    constants::GRANT_TYPES_SUPPORTED
      .iter()
      .map(|s| s.to_string())
      .collect()
  } else {
    supported_grant_types.into_iter().collect()
  };

  let token_endpoint_auth_methods_supported = constants::TOKEN_AUTH_METHODS_SUPPORTED
    .iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

  let token_endpoint_auth_signing_alg_values_supported =
    constants::TOKEN_AUTH_SIGNING_ALGS_SUPPORTED
      .iter()
      .map(|s| s.to_string())
      .collect::<Vec<_>>();

  let code_challenge_methods_supported = constants::CODE_CHALLENGE_METHODS_SUPPORTED
    .iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

  let subject_types_supported = constants::SUBJECT_TYPES_SUPPORTED
    .iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

  let id_token_signing_alg_values_supported = constants::ID_TOKEN_SIGNING_ALGS_SUPPORTED
    .iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

  let id_token_encryption_alg_values_supported: Vec<String> = Vec::new();

  let id_token_encryption_enc_values_supported: Vec<String> = Vec::new();

  let userinfo_encryption_alg_values_supported: Vec<String> = Vec::new();

  let request_object_encryption_alg_values_supported: Vec<String> = Vec::new();

  let userinfo_signing_alg_values_supported = constants::ID_TOKEN_SIGNING_ALGS_SUPPORTED
    .iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();
  let request_object_signing_alg_values_supported = constants::ID_TOKEN_SIGNING_ALGS_SUPPORTED
    .iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

  let scopes_supported = constants::SCOPES_SUPPORTED
    .iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

  let claims_supported = constants::CLAIMS_SUPPORTED
    .iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

  Ok(mises_proto::OpenIdConfiguration {
    issuer,
    authorization_endpoint,
    token_endpoint,
    userinfo_endpoint,
    revocation_endpoint,
    introspection_endpoint,
    jwks_uri,
    registration_endpoint,
    scopes_supported,
    response_types_supported,
    response_modes_supported,
    grant_types_supported,
    token_endpoint_auth_methods_supported,
    token_endpoint_auth_signing_alg_values_supported,
    code_challenge_methods_supported,
    subject_types_supported,
    id_token_signing_alg_values_supported,
    id_token_encryption_alg_values_supported,
    id_token_encryption_enc_values_supported,
    userinfo_signing_alg_values_supported,
    userinfo_encryption_alg_values_supported,
    request_object_signing_alg_values_supported,
    request_object_encryption_alg_values_supported,
    service_documentation: None,
    claims_supported,
    claims_locales_supported: Vec::new(),
    ui_locales_supported: Vec::new(),
    acr_values_supported: Vec::new(),
    claims_parameter_supported: Some(true),
    request_parameter_supported: Some(true),
    request_uri_parameter_supported: Some(false),
    require_request_uri_registration: Some(false),
    op_policy_uri: None,
    op_tos_uri: None,
    check_session_iframe,
    end_session_endpoint,
    frontchannel_logout_supported: Some(false),
    frontchannel_logout_session_supported: Some(false),
    backchannel_logout_supported: Some(false),
    backchannel_logout_session_supported: Some(false),
    device_authorization_endpoint,
    pushed_authorization_request_endpoint,
  })
}
