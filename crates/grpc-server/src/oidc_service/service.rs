use tonic::{Request, Response, Status};

use mises_core::{service::graph::GraphService, traits::Repository};

use crate::oidc_service::{authorize::authorize, constants, helpers::resolve_client_id};

pub struct OidcService<R>
where
  R: Repository,
{
  repo: R,
  issuer: String,
  public_uri: Option<url::Url>,
  hmac_secret: Option<String>,
}

impl<R> OidcService<R>
where
  R: Repository,
{
  pub fn new(
    repo: R,
    issuer: String,
    public_uri: Option<url::Url>,
    hmac_secret: Option<String>,
  ) -> Self {
    Self {
      repo,
      issuer,
      public_uri,
      hmac_secret,
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
    authorize(self.repo.clone(), req).await.map(Response::new)
  }

  async fn device_authorize(
    &self,
    _request: Request<mises_proto::DeviceAuthorizeRequest>,
  ) -> Result<Response<mises_proto::DeviceAuthorizeResponse>, Status> {
    Err(Status::unimplemented("device_authorize not implemented"))
  }

  async fn native_authenticate(
    &self,
    request: Request<mises_proto::NativeAuthenticateRequest>,
  ) -> Result<Response<mises_proto::TokenResponse>, Status> {
    let req = request.into_inner();

    let client_id = req.client_id.as_deref().unwrap_or("");
    if client_id.trim().is_empty() {
      return Err(Status::invalid_argument("client_id is required"));
    }

    let client_uuid = resolve_client_id(client_id, self.repo.clone()).await?;

    let identity_service = mises_core::service::identity::IdentityService::new(self.repo.clone());
    let user_node = match req.sub {
      Some(sub) => match identity_service.create_user(sub).await {
        Ok(n) => n,
        Err(e) => return Err(Status::internal(format!("identity service error: {}", e))),
      },
      None => match identity_service
        .create_user("desktop-admin".to_string())
        .await
      {
        Ok(n) => n,
        Err(e) => return Err(Status::internal(format!("identity service error: {}", e))),
      },
    };

    let secret = match &self.hmac_secret {
      Some(s) => s.as_bytes(),
      None => {
        return Err(Status::unimplemented(
          "native_authenticate not configured with signing secret",
        ));
      }
    };

    let now = chrono::Utc::now();
    let exp = now + chrono::Duration::seconds(3600);
    let jti = uuid::Uuid::new_v4().to_string();
    #[derive(serde::Serialize)]
    struct Claims<'a> {
      iss: &'a str,
      aud: &'a str,
      sub: String,
      exp: i64,
      iat: i64,
      jti: String,
      scope: Option<String>,
    }

    let claims = Claims {
      iss: &self.issuer,
      aud: &client_uuid.to_string(),
      sub: user_node.id.to_string(),
      exp: exp.timestamp(),
      iat: now.timestamp(),
      jti,
      scope: req.scope,
    };

    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    let token = jsonwebtoken::encode(
      &header,
      &claims,
      &jsonwebtoken::EncodingKey::from_secret(secret),
    )
    .map_err(|e| Status::internal(format!("failed to encode id_token: {}", e)))?;

    let resp = mises_proto::TokenResponse {
      access_token: String::new(),
      token_type: String::from("Bearer"),
      expires_in: Some(3600),
      refresh_token: None,
      id_token: Some(token),
      scope: None,
    };

    Ok(Response::new(resp))
  }

  async fn token(
    &self,
    _request: Request<mises_proto::TokenRequest>,
  ) -> Result<Response<mises_proto::TokenResponse>, Status> {
    Err(Status::unimplemented("token not implemented"))
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

    let response_types_supported = constants::RESPONSE_TYPES_SUPPORTED
      .iter()
      .map(|s| s.to_string())
      .collect::<Vec<_>>();

    let response_modes_supported = constants::RESPONSE_MODES
      .iter()
      .map(|s| s.to_string())
      .collect::<Vec<_>>();

    let grant_types_supported = constants::GRANT_TYPES_SUPPORTED
      .iter()
      .map(|s| s.to_string())
      .collect::<Vec<_>>();

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
