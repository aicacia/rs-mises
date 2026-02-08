use tonic::{Request, Response, Status};

use mises_core::{
  CoreError,
  model::identity::IdentityType,
  service::{graph::GraphService, identity::IdentityService},
  traits::Repository,
};
use uuid::Uuid;

/// OIDC gRPC service implementation. Uses the repository/graph service directly
/// to implement behavior (not the `mises-oidc` crate).
pub struct OidcService<R>
where
  R: Repository,
{
  repo: R,
  issuer: String,
  public_uri: Option<url::Url>,
}

impl<R> OidcService<R>
where
  R: Repository,
{
  pub fn new(repo: R, issuer: String, public_uri: Option<url::Url>) -> Self {
    Self {
      repo,
      issuer,
      public_uri,
    }
  }
}

#[tonic::async_trait]
impl<R> mises_proto::oidc_service_server::OidcService for OidcService<R>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  async fn authorize(
    &self,
    request: Request<mises_proto::AuthorizeRequest>,
  ) -> Result<Response<mises_proto::AuthorizeResponse>, Status> {
    let req = request.into_inner();

    // client_id is required
    if req.client_id.trim().is_empty() {
      return Err(Status::invalid_argument("client_id is required"));
    }

    // response_type is required and must be supported
    if req.response_type.trim().is_empty() {
      return Err(Status::invalid_argument("response_type is required"));
    }

    // support the standard response types: code, token, id_token
    let supported: [&str; 3] = ["code", "token", "id_token"];
    for part in req.response_type.split_whitespace() {
      if !supported.contains(&part) {
        return Err(Status::invalid_argument(format!(
          "unsupported response_type: {}",
          part
        )));
      }
    }

    // if redirect_uri is provided, ensure it's a valid absolute URI
    if let Some(ref redirect) = req.redirect_uri {
      if redirect.trim().is_empty() {
        return Err(Status::invalid_argument("redirect_uri provided is empty"));
      }
      if url::Url::parse(redirect).is_err() {
        return Err(Status::invalid_argument(format!(
          "invalid redirect_uri: {}",
          redirect
        )));
      }
    }

    // Validate that the client_id corresponds to an Application identity in the graph
    let client_uuid = match Uuid::parse_str(req.client_id.trim()) {
      Ok(u) => u,
      Err(_) => {
        return Err(Status::invalid_argument(format!(
          "invalid client_id: {}",
          req.client_id
        )));
      }
    };

    let identity_service = IdentityService::new(self.repo.clone());

    if let Err(e) = identity_service
      .get_node_by_id_and_identity_type(client_uuid, IdentityType::Application)
      .await
    {
      return match e {
        CoreError::NotFound => Err(Status::invalid_argument(format!(
          "client_id not found: {}",
          req.client_id
        ))),
        CoreError::InvalidInput(_) => Err(Status::invalid_argument(
          "client_id does not refer to an application",
        )),
        _ => Err(Status::internal(format!("identity service error: {}", e))),
      };
    }

    Ok(Response::new(mises_proto::AuthorizeResponse {
      redirect_uri: req.redirect_uri,
    }))
  }

  async fn token(
    &self,
    _request: Request<mises_proto::TokenRequest>,
  ) -> Result<Response<mises_proto::TokenResponse>, Status> {
    // TODO: Implement token handling using repository
    Err(Status::unimplemented("token endpoint not implemented"))
  }

  async fn device_authorize(
    &self,
    _request: Request<mises_proto::DeviceAuthorizeRequest>,
  ) -> Result<Response<mises_proto::DeviceAuthorizeResponse>, Status> {
    Err(Status::unimplemented("device_authorize not implemented"))
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
    _request: Request<mises_proto::ClientRegisterRequest>,
  ) -> Result<Response<mises_proto::Client>, Status> {
    Err(Status::unimplemented("client_register not implemented"))
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

    let response_types_supported = vec![
      String::from("code"),
      String::from("token"),
      String::from("id_token"),
    ];

    let response_modes_supported = vec![
      String::from("query"),
      String::from("fragment"),
      String::from("form_post"),
    ];

    let grant_types_supported = vec![
      String::from("authorization_code"),
      String::from("refresh_token"),
      String::from("client_credentials"),
      String::from("urn:ietf:params:oauth:grant-type:device_code"),
    ];

    let token_endpoint_auth_methods_supported = vec![
      String::from("client_secret_basic"),
      String::from("client_secret_post"),
    ];

    let token_endpoint_auth_signing_alg_values_supported = vec![String::from("ES256K")];

    let code_challenge_methods_supported = vec![String::from("S256")];

    let subject_types_supported = vec![String::from("public")];

    let id_token_signing_alg_values_supported = vec![String::from("ES256K")];

    let id_token_encryption_alg_values_supported: Vec<String> = Vec::new();

    let id_token_encryption_enc_values_supported: Vec<String> = Vec::new();

    let userinfo_encryption_alg_values_supported: Vec<String> = Vec::new();

    let request_object_encryption_alg_values_supported: Vec<String> = Vec::new();

    let userinfo_signing_alg_values_supported = vec![String::from("ES256K")];
    let request_object_signing_alg_values_supported = vec![String::from("ES256K")];

    let scopes_supported = vec![
      String::from("openid"),
      String::from("profile"),
      String::from("email"),
      String::from("offline_access"),
    ];

    let claims_supported = vec![
      String::from("iss"),
      String::from("aud"),
      String::from("exp"),
      String::from("jti"),
      String::from("scope"),
      String::from("acting_for"),
      String::from("sub"),
      String::from("name"),
      String::from("given_name"),
      String::from("family_name"),
      String::from("preferred_username"),
      String::from("email"),
      String::from("email_verified"),
      String::from("picture"),
    ];

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
    // Get keys from GraphService and convert EC public keys to JWKs
    let entries = GraphService::new(self.repo.clone())
      .list_keys()
      .await
      .map_err(|e| Status::internal(format!("list_keys error: {}", e)))?;

    let mut keys: Vec<mises_proto::Jwk> = Vec::new();

    for (id, km) in entries {
      if let Some((x, y)) = km.ec_coords_b64() {
        keys.push(mises_proto::Jwk {
          kid: id.to_string(),
          kty: String::from("EC"),
          r#use: String::from("sig"),
          alg: String::from("ES256K"),
          crv: String::from("secp256k1"),
          x,
          y,
          key_ops: vec![String::from("verify")],
        })
      }
    }

    Ok(Response::new(mises_proto::Jwks { keys }))
  }
}
