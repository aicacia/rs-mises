use tonic::Status;
use uuid::Uuid;

use mises_core::{
  model::{identity::IdentityMeta, node::NodeMeta},
  service::identity::IdentityService,
  traits::Repository,
};

use crate::{
  error::ToStatus,
  oidc_service::helpers::{ensure_application_identity, ensure_service_owns_application},
};

pub async fn client_register<R>(
  repo: &R,
  device_id: &str,
  request: mises_proto::ClientRegisterRequest,
) -> Result<mises_proto::Client, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  let client_id = request
    .client_id
    .clone()
    .and_then(|id| if id.trim().is_empty() { None } else { Some(id) });

  let service_id = request
    .service_id
    .clone()
    .filter(|id| !id.trim().is_empty())
    .ok_or_else(|| Status::invalid_argument("missing service_id"))?;

  let identity_service = IdentityService::new(repo.clone(), device_id.to_string());

  let service_node = identity_service
    .find_service_by_name(&service_id)
    .await
    .map_err(|e| e.to_status())?
    .ok_or_else(|| Status::invalid_argument(format!("service_id not found: {}", service_id)))?;
  let service_uuid = service_node.id;

  let app_node = if let Some(ref id_str) = client_id {
    let client_uuid =
      Uuid::parse_str(id_str).map_err(|_| Status::invalid_argument("invalid client_id format"))?;
    let app_node = ensure_application_identity(&identity_service, client_uuid).await?;
    ensure_service_owns_application(&identity_service, service_uuid, app_node.id).await?;
    app_node
  } else {
    let app_name = request
      .name
      .clone()
      .filter(|n| !n.trim().is_empty())
      .unwrap_or_else(|| "OIDC Client".to_string());

    let (app_node, _key_node) = identity_service
      .create_application(app_name, None)
      .await
      .map_err(|e| e.to_status())?;

    identity_service
      .set_owner(service_uuid, app_node.id)
      .await
      .map_err(|e| e.to_status())?;

    app_node
  };

  let app_id = app_node.id;

  let current_oidc = if let NodeMeta::Identity(identity_meta) = &app_node.metadata {
    if let IdentityMeta::Application { oidc, .. } = identity_meta.as_ref() {
      oidc.as_ref().clone()
    } else {
      None
    }
  } else {
    None
  };

  let mut oidc_meta = current_oidc.unwrap_or_default();

  if oidc_meta.client_id.is_none() {
    oidc_meta.client_id = Some(app_id.to_string());
  }

  if let Some(secret) = request.client_secret.filter(|s| !s.trim().is_empty()) {
    oidc_meta.client_secret = Some(secret);
  }

  if !request.redirect_uris.is_empty() {
    oidc_meta.redirect_uris = request.redirect_uris;
  }

  if !request.grant_types.is_empty() {
    oidc_meta.grant_types = request
      .grant_types
      .into_iter()
      .filter_map(|gt| gt.parse().ok())
      .collect();
  }

  if !request.response_types.is_empty() {
    oidc_meta.response_types = request
      .response_types
      .into_iter()
      .filter_map(|rt| rt.parse().ok())
      .collect();
  }

  if let Some(scope) = request.scope.filter(|s| !s.trim().is_empty()) {
    oidc_meta.scope = scope;
  }

  if let Some(auth_method) = request
    .token_endpoint_auth_method
    .filter(|m| !m.trim().is_empty())
    && let Ok(method) = auth_method.parse()
  {
    oidc_meta.token_endpoint_auth_method = method;
  }

  if let Some(application_urn) = request.application_urn.filter(|u| !u.trim().is_empty()) {
    oidc_meta.application_urn = application_urn;
  }

  if let Some(name) = request.name.filter(|n| !n.trim().is_empty()) {
    oidc_meta.client_name = name;
  }

  let updated_identity = IdentityMeta::Application {
    name: if let NodeMeta::Identity(identity_meta) = &app_node.metadata {
      if let IdentityMeta::Application { name, .. } = identity_meta.as_ref() {
        name.clone()
      } else {
        "OIDC Client".to_string()
      }
    } else {
      "OIDC Client".to_string()
    },
    oidc: Box::new(Some(oidc_meta.clone())),
  };

  repo
    .update_node(app_id, NodeMeta::Identity(Box::new(updated_identity)), None)
    .await
    .map_err(|e| e.to_status())?;

  Ok(mises_proto::Client {
    id: app_id.to_string(),
    client_id: oidc_meta.client_id.unwrap_or_else(|| app_id.to_string()),
    client_secret: oidc_meta.client_secret,
    name: if oidc_meta.client_name.is_empty() {
      None
    } else {
      Some(oidc_meta.client_name)
    },
    redirect_uris: oidc_meta.redirect_uris,
    grant_types: oidc_meta
      .grant_types
      .iter()
      .map(|gt: &mises_core::model::oidc::GrantType| gt.as_str().to_string())
      .collect(),
    response_types: oidc_meta
      .response_types
      .iter()
      .map(|rt: &mises_core::model::oidc::ResponseType| rt.as_str().to_string())
      .collect(),
    scope: if oidc_meta.scope.is_empty() {
      None
    } else {
      Some(oidc_meta.scope)
    },
    token_endpoint_auth_method: Some(oidc_meta.token_endpoint_auth_method.as_str().to_string()),
  })
}
