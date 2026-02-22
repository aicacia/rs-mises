use tonic::{Request, Response, Status};

use mises_core::{
  model::{
    identity::{IdentityMeta, IdentityType},
    node::NodeMeta,
  },
  service::identity::IdentityService,
  traits::Repository,
};

pub struct ClientService<R>
where
  R: Repository,
{
  repo: R,
  device_id: String,
}

impl<R> ClientService<R>
where
  R: Repository,
{
  pub fn new(repo: R, device_id: String) -> Self {
    Self { repo, device_id }
  }
}

#[tonic::async_trait]
impl<R> mises_proto::client_service_server::ClientService for ClientService<R>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  async fn get(
    &self,
    request: Request<mises_proto::GetClientRequest>,
  ) -> Result<Response<mises_proto::Client>, Status> {
    let request = request.into_inner();

    let client_id = uuid::Uuid::parse_str(&request.client_id)
      .map_err(|_| Status::invalid_argument("Invalid client ID"))?;

    let identity_service = IdentityService::new(self.repo.clone(), self.device_id.clone());

    let client = identity_service
      .get_node_by_id_and_identity_type(client_id, IdentityType::Application)
      .await
      .map_err(|_| Status::not_found("Client not found"))?;

    let oidc_client = match &client.metadata {
      NodeMeta::Identity(boxed_meta) => {
        if let IdentityMeta::Application { oidc, .. } = boxed_meta.as_ref() {
          oidc
            .as_ref()
            .as_ref()
            .ok_or_else(|| Status::not_found("Client OIDC metadata not found"))?
        } else {
          return Err(Status::not_found("Client not found"));
        }
      }
      _ => return Err(Status::not_found("Client not found")),
    };

    Ok(Response::new(mises_proto::Client {
      id: client.id.to_string(),
      client_id: oidc_client
        .client_id
        .clone()
        .unwrap_or_else(|| client.id.to_string()),
      client_secret: oidc_client.client_secret.clone(),
      name: Some(oidc_client.client_name.clone()),
      redirect_uris: oidc_client.redirect_uris.clone(),
      grant_types: oidc_client
        .grant_types
        .iter()
        .map(|gt| gt.as_str().to_string())
        .collect(),
      response_types: oidc_client
        .response_types
        .iter()
        .map(|rt| rt.as_str().to_string())
        .collect(),
      scope: Some(oidc_client.scope.clone()),
      token_endpoint_auth_method: None,
      require_pkce: None,
      application_type: None,
      contacts: vec![],
      service_id: oidc_client.service_id.clone().into(),
      client_uri: None,
      logo_uri: None,
      policy_uri: None,
      tos_uri: None,
      jwks_uri: None,
      jwks: None,
      sector_identifier_uri: None,
      subject_type: None,
      id_token_signed_response_alg: None,
      id_token_encrypted_response_alg: None,
      id_token_encrypted_response_enc: None,
      userinfo_signed_response_alg: None,
      userinfo_encrypted_response_alg: None,
      userinfo_encrypted_response_enc: None,
      request_object_signing_alg: None,
      request_object_encryption_alg: None,
      request_object_encryption_enc: None,
      token_endpoint_auth_signing_alg: None,
      default_max_age: None,
      require_auth_time: None,
      default_acr_values: vec![],
      initiate_login_uri: None,
      request_uris: vec![],
      post_logout_redirect_uris: vec![],
      frontchannel_logout_uri: None,
      frontchannel_logout_session_required: None,
      backchannel_logout_uri: None,
      backchannel_logout_session_required: None,
      access_token_expiry: None,
      refresh_token_expiry: None,
    }))
  }

  async fn is_allowed_for_user(
    &self,
    _request: Request<mises_proto::IsAllowedForUserRequest>,
  ) -> Result<Response<mises_proto::ClientAllowed>, Status> {
    unimplemented!()
  }

  async fn approve_for_user(
    &self,
    _request: Request<mises_proto::ApproveForUserRequest>,
  ) -> Result<Response<()>, Status> {
    unimplemented!()
  }
}
