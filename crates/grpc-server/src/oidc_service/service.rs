use std::collections::HashSet;

use mises_core::{
  model::node::NodeMeta,
  service::{graph::GraphService, identity::IdentityService},
  traits::Repository,
};
use mises_graph::KeyValueStoreExecutor;
use tonic::{Request, Response, Status};
use url::Url;

use crate::{
  jwt::extract_and_parse_jwt_claims,
  oidc_service::{authorize::authorize, client_register::client_register, constants, token::token},
};

pub struct OidcService<R, S>
where
  R: Repository,
  S: KeyValueStoreExecutor,
{
  repo: R,
  store: S,
  issuer: String,
  public_uri: Option<Url>,
  sign_in_url: Option<String>,
}

impl<R, S> OidcService<R, S>
where
  R: Repository,
  S: KeyValueStoreExecutor,
{
  pub fn new(
    repo: R,
    store: S,
    issuer: String,
    public_uri: Option<Url>,
    sign_in_url: Option<String>,
  ) -> Self {
    Self {
      repo,
      store,
      issuer,
      public_uri,
      sign_in_url,
    }
  }
}

#[tonic::async_trait]
impl<R, S> mises_proto::oidc_service_server::OidcService for OidcService<R, S>
where
  R: Repository + Clone + Send + Sync + 'static,
  S: KeyValueStoreExecutor + Clone + Send + Sync + 'static,
{
  async fn authorize(
    &self,
    request: Request<mises_proto::AuthorizeRequest>,
  ) -> Result<Response<mises_proto::AuthorizeResponse>, Status> {
    log::debug!("authorize request: {:?}", request);

    let claims = if let Some(auth_header) = request
      .metadata()
      .get("authorization")
      .and_then(|v| v.to_str().ok())
    {
      Some(
        extract_and_parse_jwt_claims(auth_header)
          .map_err(|_| Status::unauthenticated("invalid bearer token"))?,
      )
    } else {
      None
    };

    authorize(
      &self.repo,
      &self.store,
      request.into_inner(),
      claims,
      &self.sign_in_url,
    )
    .await
    .map(Response::new)
  }

  async fn device_authorize(
    &self,
    _request: Request<mises_proto::DeviceAuthorizeRequest>,
  ) -> Result<Response<mises_proto::DeviceAuthorizeResponse>, Status> {
    Err(Status::unimplemented("device_authorize not implemented"))
  }

  async fn token(
    &self,
    request: Request<mises_proto::TokenRequest>,
  ) -> Result<Response<mises_proto::TokenResponse>, Status> {
    log::debug!("Token request: {:?}", request);

    let claims = if let Some(auth_header) = request
      .metadata()
      .get("authorization")
      .and_then(|v| v.to_str().ok())
    {
      Some(
        extract_and_parse_jwt_claims(auth_header)
          .map_err(|_| Status::unauthenticated("invalid bearer token"))?,
      )
    } else {
      None
    };

    token(
      &self.repo,
      &self.store,
      request.into_inner(),
      claims,
      &self.issuer,
    )
    .await
    .map(Response::new)
  }

  async fn introspect(
    &self,
    _request: Request<mises_proto::IntrospectRequest>,
  ) -> Result<Response<mises_proto::IntrospectResponse>, Status> {
    Err(Status::unimplemented("introspect not implemented"))
  }

  async fn revoke(
    &self,
    _request: Request<mises_proto::RevokeRequest>,
  ) -> Result<Response<()>, Status> {
    Err(Status::unimplemented("revoke not implemented"))
  }

  async fn pushed_authorize(
    &self,
    _request: Request<mises_proto::PushedAuthorizeRequest>,
  ) -> Result<Response<mises_proto::PushedAuthorizeResponse>, Status> {
    Err(Status::unimplemented("pushed_authorize not implemented"))
  }

  async fn backchannel_auth(
    &self,
    _request: Request<mises_proto::BackchannelAuthRequest>,
  ) -> Result<Response<mises_proto::BackchannelAuthResponse>, Status> {
    Err(Status::unimplemented("backchannel_auth not implemented"))
  }

  async fn client_register(
    &self,
    request: Request<mises_proto::ClientRegisterRequest>,
  ) -> Result<Response<mises_proto::Client>, Status> {
    log::debug!("client_register request: {:?}", request);

    let claims = if let Some(auth_header) = request
      .metadata()
      .get("authorization")
      .and_then(|v| v.to_str().ok())
    {
      Some(
        extract_and_parse_jwt_claims(auth_header)
          .map_err(|_| Status::unauthenticated("invalid bearer token"))?,
      )
    } else {
      None
    };

    if claims.is_none() {
      return Err(Status::unauthenticated(
        "authorization required: bearer token not provided",
      ));
    }

    client_register(&self.repo, request.into_inner())
      .await
      .map(Response::new)
  }

  async fn end_session(
    &self,
    _request: Request<mises_proto::EndSessionRequest>,
  ) -> Result<Response<mises_proto::EndSessionResponse>, Status> {
    Err(Status::unimplemented("end_session not implemented"))
  }

  async fn get_user_info(
    &self,
    _request: Request<mises_proto::UserInfoRequest>,
  ) -> Result<Response<mises_proto::UserInfo>, Status> {
    Err(Status::unimplemented("get_user_info not implemented"))
  }

  async fn get_open_id_configuration(
    &self,
    _request: Request<()>,
  ) -> Result<Response<mises_proto::OpenIdConfiguration>, Status> {
    let issuer = self.issuer.to_owned();

    let identity_service = IdentityService::new(self.repo.clone());
    let applications = identity_service
      .list_applications()
      .await
      .map_err(|e| Status::internal(format!("list_applications error: {}", e)))?;

    let mut supported_response_types: HashSet<String> = Default::default();
    let mut supported_grant_types: HashSet<String> = Default::default();

    for app in applications {
      if let NodeMeta::Identity(mises_core::model::identity::IdentityMeta::Application {
        oidc: Some(oidc),
        ..
      }) = &app.metadata
      {
        for rt in &oidc.response_types {
          supported_response_types.insert(rt.as_str().to_string());
        }
        for gt in &oidc.grant_types {
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
    ) = if let Some(public_uri) = &self.public_uri {
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

    Ok(Response::new(mises_proto::OpenIdConfiguration {
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
    }))
  }

  async fn get_jwks(&self, _request: Request<()>) -> Result<Response<mises_proto::Jwks>, Status> {
    let entries = GraphService::new(self.repo.clone())
      .list_keys()
      .await
      .map_err(|e| Status::internal(format!("list_keys error: {}", e)))?;

    let mut keys: Vec<mises_proto::Jwk> = Vec::new();

    for (id, km) in entries {
      if let Some(x) = km.ec_coords_b64() {
        keys.push(mises_proto::Jwk {
          kid: id.to_string(),
          kty: String::from("OKP"),
          r#use: String::from("sig"),
          alg: String::from("EdDSA"),
          crv: String::from("Ed25519"),
          x,
          y: String::new(),
          key_ops: vec![String::from("verify")],
        })
      }
    }

    Ok(Response::new(mises_proto::Jwks { keys }))
  }
}
