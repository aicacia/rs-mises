use tonic::Status;
use uuid::Uuid;

use mises_core::{
  error::CoreError,
  model::{
    identity::{IdentityMeta, IdentityType},
    node::NodeMeta,
  },
  service::identity::IdentityService,
  traits::Repository,
};

use crate::{
  error::ToStatus,
  helpers::{OptionExt, ResultExt},
  oidc_service::helpers::ensure_service_owns_application,
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
    .and_then(|id| if id.trim().is_empty() { None } else { Some(id) });

  let service_id = request
    .service_id
    .clone()
    .filter(|id| !id.trim().is_empty())
    .or_invalid_argument("missing service_id")?;

  let identity_service = IdentityService::new(repo.clone(), device_id.to_string());

  let service_node = identity_service
    .find_service_by_name(&service_id)
    .await
    .map_err(|e| e.to_status())?
    .ok_or_else(|| Status::invalid_argument(format!("service_id not found: {}", service_id)))?;
  let service_uuid = service_node.id;

  let is_new_client;
  let app_node = if let Some(ref id_str) = client_id {
    let client_uuid = Uuid::parse_str(id_str).or_invalid_argument("invalid client_id format")?;

    match identity_service
      .get_node_by_id_and_identity_type(client_uuid, IdentityType::Application)
      .await
    {
      Ok(existing_node) => {
        ensure_service_owns_application(&identity_service, service_uuid, existing_node.id).await?;
        is_new_client = false;
        existing_node
      }
      Err(CoreError::NotFound) => {
        let app_name = request
          .name
          .clone()
          .filter(|n| !n.trim().is_empty())
          .unwrap_or_else(|| "OIDC Client".to_string());

        let mut oidc_meta = mises_core::model::oidc::OidcClientMeta {
          client_name: app_name.clone(),
          ..Default::default()
        };
        oidc_meta.client_id = client_uuid.to_string();

        let (app_node, _key_node) = identity_service
          .create_application(Some(client_uuid), oidc_meta)
          .await
          .map_err(|e| e.to_status())?;

        identity_service
          .set_owner(service_uuid, app_node.id)
          .await
          .map_err(|e| e.to_status())?;

        is_new_client = true;
        app_node
      }
      Err(CoreError::InvalidInput(_)) => {
        return Err(Status::invalid_argument(
          "invalid_request: client_id does not refer to an application",
        ));
      }
      Err(e) => return Err(Status::internal(format!("identity service error: {}", e))),
    }
  } else {
    let app_name = request
      .name
      .clone()
      .filter(|n| !n.trim().is_empty())
      .unwrap_or_else(|| "OIDC Client".to_string());

    let oidc_meta = mises_core::model::oidc::OidcClientMeta {
      client_name: app_name.clone(),
      ..Default::default()
    };

    let (app_node, _key_node) = identity_service
      .create_application(None, oidc_meta)
      .await
      .map_err(|e| e.to_status())?;

    identity_service
      .set_owner(service_uuid, app_node.id)
      .await
      .map_err(|e| e.to_status())?;

    is_new_client = true;
    app_node
  };

  let app_id = app_node.id;

  let current_oidc = if let NodeMeta::Identity(identity_meta) = &app_node.metadata {
    if let IdentityMeta::Application { oidc } = identity_meta.as_ref() {
      oidc.as_ref().clone()
    } else {
      Default::default()
    }
  } else {
    Default::default()
  };

  let mut oidc_meta = current_oidc;

  if is_new_client {
    if oidc_meta.client_id.is_empty() {
      oidc_meta.client_id = app_id.to_string();
    }
    oidc_meta.client_secret = Uuid::new_v4().to_string();
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

  oidc_meta.service_id = service_id;

  if let Some(name) = request.name.filter(|n| !n.trim().is_empty()) {
    oidc_meta.client_name = name;
  }

  let updated_identity = IdentityMeta::Application {
    oidc: Box::new(oidc_meta.clone()),
  };

  repo
    .update_node(app_id, NodeMeta::Identity(Box::new(updated_identity)), None)
    .await
    .map_err(|e| e.to_status())?;

  Ok(mises_proto::Client {
    id: app_id.to_string(),
    client_id: if oidc_meta.client_id.is_empty() {
      app_id.to_string()
    } else {
      oidc_meta.client_id
    },
    client_secret: if oidc_meta.client_secret.is_empty() {
      None
    } else {
      Some(oidc_meta.client_secret)
    },
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
    require_pkce: None,
    application_type: None,
    contacts: vec![],
    service_id: Some(oidc_meta.service_id),
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
  })
}
