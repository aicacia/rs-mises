use tonic::{Request, Response, Status};

use mises_core::service::graph::GraphService;
use mises_core::traits::Repository;

/// OIDC gRPC service implementation. Uses the repository/graph service directly
/// to implement behavior (not the `mises-oidc` crate).
pub struct OidcService<R>
where
  R: Repository,
{
  graph_service: GraphService<R>,
  issuer: String,
  public_uri: Option<url::Url>,
}

impl<R> OidcService<R>
where
  R: Repository,
{
  pub fn new(graph_service: GraphService<R>, issuer: String, public_uri: Option<url::Url>) -> Self {
    Self {
      graph_service,
      issuer,
      public_uri,
    }
  }
}

#[tonic::async_trait]
impl<R> mises_proto::oidc_service_server::OidcService for OidcService<R>
where
  R: Repository + Send + Sync + 'static,
{
  async fn authorize(
    &self,
    _request: Request<mises_proto::AuthorizeRequest>,
  ) -> Result<Response<mises_proto::AuthorizeResponse>, Status> {
    // TODO: Implement using repository/graph service
    Err(Status::unimplemented("authorize not implemented"))
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

    let (jwks_uri, authorization_endpoint, token_endpoint, userinfo_endpoint, end_session_endpoint) =
      if let Some(public_uri) = &self.public_uri {
        (
          public_uri.join("/jwks.json").map(|u| u.to_string()).ok(),
          public_uri.join("/authorize").map(|u| u.to_string()).ok(),
          public_uri.join("/token").map(|u| u.to_string()).ok(),
          public_uri.join("/user-info").map(|u| u.to_string()).ok(),
          public_uri.join("/end_session").map(|u| u.to_string()).ok(),
        )
      } else {
        (None, None, None, None, None)
      };

    let response_types_supported = vec![
      String::from("code"),
      String::from("token"),
      String::from("id_token"),
    ];
    let subject_types_supported = vec![String::from("public")];
    let id_token_signing_alg_values_supported = vec![String::from("ES256K")];

    Ok(Response::new(mises_proto::OpenIdConfiguration {
      issuer,
      jwks_uri,
      authorization_endpoint,
      token_endpoint,
      userinfo_endpoint,
      end_session_endpoint,
      response_types_supported,
      subject_types_supported,
      id_token_signing_alg_values_supported,
    }))
  }

  async fn get_jwks(&self, _request: Request<()>) -> Result<Response<mises_proto::Jwks>, Status> {
    // Get keys from GraphService and convert EC public keys to JWKs
    let entries = self
      .graph_service
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
