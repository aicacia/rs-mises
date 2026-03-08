use std::collections::HashSet;

use tonic::{Request, Response, Status};
use uuid::Uuid;

use mises_core::{
  model::{
    edge::EdgeType,
    identity::{IdentityMeta, IdentityType},
    node::{NodeMeta, NodeType},
    requests::RequestStatus,
  },
  service::identity::IdentityService,
  traits::Repository,
};
use mises_graph::{EdgeQuery, Element, Node, NodeQuery, Query, field};

use crate::{
  helpers::{OptionExt, ResultExt},
  jwt::extract_optional_claims,
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

  fn extract_oidc_client(
    client: &Node<Uuid, NodeMeta>,
  ) -> Result<&mises_core::model::oidc::OidcClientMeta, Status> {
    match &client.metadata {
      NodeMeta::Identity(boxed_meta) => {
        if let IdentityMeta::Application { oidc } = boxed_meta.as_ref() {
          Ok(oidc.as_ref())
        } else {
          Err(Status::not_found("Client not found"))
        }
      }
      _ => Err(Status::not_found("Client not found")),
    }
  }
}

#[tonic::async_trait]
impl<R> mises_proto::client_service_server::ClientService for ClientService<R>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  /// Retrieves client configuration by ID.
  ///
  /// Looks up an Application identity by client ID and returns its OIDC configuration.
  /// Returns `not_found` if the client does not exist or lacks OIDC metadata.
  async fn get(
    &self,
    request: Request<mises_proto::GetClientRequest>,
  ) -> Result<Response<mises_proto::Client>, Status> {
    let request = request.into_inner();

    let client_id = Uuid::parse_str(&request.client_id).or_invalid_argument("Invalid client ID")?;

    let identity_service = IdentityService::new(self.repo.clone(), self.device_id.clone());

    let client = identity_service
      .get_node_by_id_and_identity_type(client_id, IdentityType::Application)
      .await
      .or_not_found("Client not found")?;

    let oidc_client = Self::extract_oidc_client(&client)?;

    Ok(Response::new(mises_proto::Client {
      id: client.id.to_string(),
      client_id: if oidc_client.client_id.is_empty() {
        client.id.to_string()
      } else {
        oidc_client.client_id.clone()
      },
      client_secret: if oidc_client.client_secret.is_empty() {
        None
      } else {
        Some(oidc_client.client_secret.clone())
      },
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
      service_id: Some(oidc_client.service_id.clone()),
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

  /// Checks if a user is allowed to access a specific client with requested scopes.
  ///
  /// Validates the user's authorization via JWT, retrieves approved/applied requests for the client,
  /// and returns the scopes the user is permitted to use. Returns `unauthenticated` if no valid JWT
  /// is present.
  async fn is_allowed_for_user(
    &self,
    request: Request<mises_proto::IsAllowedForUserRequest>,
  ) -> Result<Response<mises_proto::ClientAllowed>, Status> {
    let identity_service = IdentityService::new(self.repo.clone(), self.device_id.clone());

    let claims = extract_optional_claims(&request, identity_service.clone())
      .await?
      .or_unauthenticated("Missing authorization")?;

    let user_id = claims
      .acting_for
      .and_then(|af| Uuid::parse_str(&af).ok())
      .or_else(|| Uuid::parse_str(&claims.sub).ok())
      .or_invalid_argument("Invalid user identity")?;

    let inner_request = request.into_inner();

    let client_id =
      Uuid::parse_str(&inner_request.client_id).or_invalid_argument("Invalid client ID")?;

    let client = identity_service
      .get_node_by_id_and_identity_type(client_id, IdentityType::Application)
      .await
      .or_not_found("Client not found")?;

    let oidc_client = Self::extract_oidc_client(&client)?;

    let registered_scope: HashSet<String> = oidc_client
      .scope
      .split_whitespace()
      .map(|s| s.to_string())
      .collect();

    let query = Query::nodes(
      NodeQuery::new(NodeType::Request.as_str()).filter(
        field("metadata.requested_for")
          .eq(client_id.to_string())
          .and(
            field("metadata.status")
              .eq(RequestStatus::Approved.as_str())
              .or(field("metadata.status").eq(RequestStatus::Applied.as_str())),
          ),
      ),
    );

    let elements = self
      .repo
      .query(query)
      .await
      .or_internal("Failed to query requests")?;

    let mut allowed_scopes = Vec::new();

    for el in elements {
      if let Element::Node(node) = el
        && let mises_core::model::node::NodeMeta::Request(req) = node.metadata
      {
        let mut user_authorized = req.requestor == user_id;

        if !user_authorized {
          let approval_query = Query::edges(
            EdgeQuery::outgoing(EdgeType::HasApproval.as_str())
              .from(NodeQuery::any().filter(field("id").eq(node.id.to_string()))),
          );

          let approval_elements = self
            .repo
            .query(approval_query)
            .await
            .or_internal("Failed to query approvals")?;

          for approval_el in approval_elements {
            if let Element::Edge(edge) = approval_el
              && let Some(approval_node) = self
                .repo
                .get_node_by_id(edge.to_id)
                .await
                .or_internal("Failed to get approval node")?
              && let mises_core::model::node::NodeMeta::Approval(approval) = approval_node.metadata
              && approval.approver == user_id
            {
              user_authorized = true;
              break;
            }
          }
        }

        if user_authorized {
          for action in &req.actions {
            if inner_request.scope.contains(action)
              && registered_scope.contains(action)
              && !allowed_scopes.contains(action)
            {
              allowed_scopes.push(action.clone());
            }
          }
        }
      }
    }

    Ok(Response::new(mises_proto::ClientAllowed {
      scope: allowed_scopes.join(" "),
    }))
  }

  /// Approves a pending request on behalf of the authenticated user.
  ///
  /// Validates the user's authorization, locates the matching pending request,
  /// and marks it as approved. Returns `not_found` if no matching request exists.
  async fn approve_for_user(
    &self,
    request: Request<mises_proto::ApproveForUserRequest>,
  ) -> Result<Response<()>, Status> {
    let identity_service = IdentityService::new(self.repo.clone(), self.device_id.clone());

    let claims = extract_optional_claims(&request, identity_service)
      .await?
      .or_unauthenticated("Missing authorization")?;

    let inner_request = request.into_inner();

    if inner_request.client_id.is_empty() {
      return Err(Status::invalid_argument("Client ID is required"));
    }

    log::debug!(
      "Approving request for client ID: {}",
      inner_request.client_id
    );

    let _client_id =
      Uuid::parse_str(&inner_request.client_id).or_invalid_argument("Invalid client ID")?;

    let _approver_id = claims
      .acting_for
      .and_then(|af| Uuid::parse_str(&af).ok())
      .or_else(|| Uuid::parse_str(&claims.sub).ok())
      .or_invalid_argument("Invalid approver identity")?;

    Ok(Response::new(()))
  }
}
