use tonic::{Request, Response, Status};
use url::Url;

use mises_core::{service::graph::GraphService, traits::Repository};
use mises_graph::KeyValueStoreExecutor;

use crate::oidc_service::{
  authorize::authorize, client_register::client_register, helpers::extract_optional_claims,
  open_id_configuration::get_open_id_configuration, token::token,
};

pub struct OidcService<R, S>
where
  R: Repository,
  S: KeyValueStoreExecutor,
{
  repo: R,
  device_id: String,
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
    device_id: String,
    store: S,
    issuer: String,
    public_uri: Option<Url>,
    sign_in_url: Option<String>,
  ) -> Self {
    Self {
      repo,
      device_id,
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

    let claims = extract_optional_claims(&request)?;

    authorize(
      &self.repo,
      &self.device_id,
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

    let claims = extract_optional_claims(&request)?;

    token(
      &self.repo,
      &self.device_id,
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

    let claims = extract_optional_claims(&request)?;

    if claims.is_none() {
      return Err(Status::unauthenticated(
        "authorization required: bearer token not provided",
      ));
    }

    client_register(&self.repo, &self.device_id, request.into_inner())
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
    get_open_id_configuration::<R, S>(
      &self.repo,
      &self.device_id,
      self.issuer.to_owned(),
      &self.public_uri,
    )
    .await
    .map(Response::new)
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
